//! Dedicated Settings entries for the reviewed gen 3 PIE. The original
//! tables stay in place for any callers that use the adjacent stock data.
use super::{
    discovery,
    elf::{align, bytes, word, Elf},
    menu::{self, Section},
};
use crate::{device, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const STOCK_SHA: &str = "3f1c68d3ab1a86dc3ea1807c7c0bd0e0e33475efb895d6cb17c5389bcabc94d2";
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_GNU_RELRO: u32 = 0x6474_e552;
const R_ARM_RELATIVE: u32 = 23;
const DT_REL: u32 = 17;
const DT_RELSZ: u32 = 18;
const DT_RELENT: u32 = 19;
const DT_RELCOUNT: u32 = 0x6fff_fffa;

#[derive(Clone, Copy)]
struct Reference {
    literal: u32,
    add: u32,
    register: u32,
}

fn references(elf: &Elf, target: u32) -> Result<Vec<Reference>> {
    let text = elf.section(".text")?;
    let data = elf.contents(text);
    let mut result = Vec::new();
    // The reviewed PIE loads a PC-relative delta, then adds it to the PC.
    for (index, bytes) in data.chunks_exact(4).enumerate() {
        let load = u32::from_le_bytes(bytes.try_into().unwrap());
        if load & 0xff7f_0000 != 0xe51f_0000 {
            continue;
        }
        let pc = text.address + 4 * index as u32 + 8;
        let delta = load & 0xfff;
        let literal = if load & (1 << 23) != 0 { pc.wrapping_add(delta) } else { pc.wrapping_sub(delta) };
        let Ok(offset) = elf.read(literal) else { continue };
        let register = (load >> 12) & 15;
        for next in index + 1..(index + 16).min(data.len() / 4) {
            let instruction = word(data, 4 * next)?;
            let add = text.address + 4 * next as u32;
            if instruction == 0xe08f_0000 | (register << 12) | register && offset.wrapping_add(add + 8) == target {
                result.push(Reference { literal, add, register });
            }
        }
    }
    Ok(result)
}

fn one<T>(items: impl IntoIterator<Item = T>, what: &str) -> Result<T> {
    let mut iter = items.into_iter();
    match (iter.next(), iter.next()) {
        (Some(value), None) => Ok(value),
        _ => device(format!("expanded Settings menu needs one {what}")),
    }
}

fn put(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn redirect(elf: &Elf, output: &mut [u8], reference: Reference, address: u32) -> Result<()> {
    put(output, elf.offset(reference.literal)?, address.wrapping_sub(reference.add + 8));
    Ok(())
}

fn registry_ref(elf: &Elf, address: u32, following: u32) -> Result<Reference> {
    let refs = references(elf, address)?;
    if refs.len() != 2 || refs[0].literal == refs[1].literal {
        return device("unexpected uses of the stock screen registry");
    }
    let registration = one(
        refs.iter()
            .copied()
            .filter(|r| elf.read(r.add + 4).is_ok_and(|w| w & 0xffff_0fff == 0xe590_0000 | (r.register << 16))),
        "screen registration loop",
    )?;
    // The other reference advances past the sentinel to another stock list.
    let other = refs.iter().find(|r| r.literal != registration.literal).unwrap();
    if elf.read(other.add + 4)? & 0xffff_0fff != 0xe590_0000 | (other.register << 16) | following {
        return device("the second screen registry reference did not skip the sentinel");
    }
    Ok(registration)
}

fn menu_refs(elf: &Elf, address: u32, count: usize) -> Result<(Reference, Reference, u32, u32)> {
    let bound = menu::bound(elf, address, count)?;
    let refs = references(elf, address)?;
    if refs.len() != 2 || refs[0].literal == refs[1].literal {
        return device("unexpected uses of the stock Settings list");
    }
    let build =
        one(refs.iter().copied().filter(|r| r.add < bound && bound - r.add <= 16), "Settings construction loop")?;
    let lookup = *refs.iter().find(|r| r.literal != build.literal).unwrap();
    let mut limit = Vec::new();
    let mut loads_entries = false;
    for index in 1..17 {
        let at = lookup.add + 4 * index;
        let instruction = elf.read(at)?;
        // This loop compares its zero-based index with the last stock index.
        if instruction & 0xfff0_0000 == 0xe350_0000 && instruction & 0xfff == count as u32 - 1 {
            limit.push(at);
        }
        if instruction & 0xffff_0fff == 0xe490_0004 | (lookup.register << 16) {
            loads_entries = true;
        }
    }
    if !loads_entries {
        return device("could not verify the Settings lookup's list traversal");
    }
    Ok((build, lookup, bound, one(limit, "Settings lookup bound")?))
}

/// Copies the writable segment to a later file offset and puts the new tables
/// in the virtual gap before it. The existing contents keep their addresses
/// and the original RELRO range grows to protect the new tables after loading.
pub(super) fn install(
    elf: &Elf,
    output: &mut Vec<u8>,
    headers: &mut [[u32; 8]],
    sections: &[Section],
    extra_entries: &[&str],
    factory: u32,
) -> Result<serde_json::Value> {
    let (menu_address, stock_names) = discovery::settings(elf)?;
    let registry = discovery::registry(elf)?;
    let registry_size = registry.names.len() as u32 * 8;
    if !registry.code_first
        || registry_size + 8 > 0xfff
        || elf.read(registry.address + registry_size)? != 0
        || elf.read(registry.address + registry_size + 4)? != 0
    {
        return device("unexpected stock screen registration layout");
    }
    let register = registry_ref(elf, registry.address, registry_size + 8)?;
    let (menu_build, menu_lookup, bound, lookup_bound) = menu_refs(elf, menu_address, stock_names.len())?;
    let mut names = menu::arranged(&stock_names, sections)?;
    let padded_count = stock_names.len() + extra_entries.len();
    if padded_count * 4 > 255 {
        return device("expanded Settings list outgrew the reviewed loop bounds");
    }

    let rw = one(
        headers.iter().enumerate().filter(|(_, h)| h[0] == PT_LOAD && h[6] == 6).map(|(i, _)| i),
        "writable load segment",
    )?;
    let relro = one(headers.iter().enumerate().filter(|(_, h)| h[0] == PT_GNU_RELRO).map(|(i, _)| i), "RELRO segment")?;
    let dynamic =
        one(headers.iter().enumerate().filter(|(_, h)| h[0] == PT_DYNAMIC).map(|(i, _)| i), "dynamic segment")?;
    let old = headers[rw];
    let alignment = old[7] as usize;
    if !alignment.is_power_of_two() || headers[relro][1..4] != old[1..4] || headers[dynamic][1] < old[1] {
        return device("unexpected writable segment layout");
    }
    let prefix_address = old[2] & !(old[7] - 1);
    let prefix_size = (old[2] - prefix_address) as usize;
    let menu_size = padded_count * 4;
    let registry_at = menu_size;
    let names_at = registry_at + registry_size as usize + (extra_entries.len() + 1) * 8;
    if prefix_size < names_at
        || headers
            .iter()
            .enumerate()
            .any(|(i, h)| i != rw && h[0] == PT_LOAD && h[2] < old[2] && h[2].saturating_add(h[5]) > prefix_address)
    {
        return device("the writable segment has no safe prefix for expanded Settings tables");
    }
    let new_registry = prefix_address + registry_at as u32;

    let rel = elf.section(".rel.dyn")?;
    let old_rel = elf.contents(rel);
    let mut original_slots = BTreeSet::new();
    for entry in old_rel.chunks_exact(8) {
        if word(entry, 4)? == R_ARM_RELATIVE {
            original_slots.insert(word(entry, 0)?);
        }
    }
    let mut prefix = vec![0u8; prefix_size];
    let mut relocations = Vec::new();
    let mut extra_pointers = BTreeMap::new();
    let mut name_at = names_at;
    for (index, &name) in extra_entries.iter().enumerate() {
        if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
            return device("invalid custom Settings screen name");
        }
        let pointer = prefix_address + name_at as u32;
        if registry.names.iter().any(|stock| stock == name) || extra_pointers.insert(name, pointer).is_some() {
            return device(format!("duplicate Settings screen name {name}"));
        }
        let end = name_at + name.len() + 1;
        let Some(buffer) = prefix.get_mut(name_at..end) else {
            return device("the writable segment has no room for the custom Settings screen names");
        };
        buffer[..name.len()].copy_from_slice(name.as_bytes());
        name_at = end;
        let slot = registry_size + index as u32 * 8;
        put(&mut prefix, registry_at + slot as usize, factory);
        put(&mut prefix, registry_at + slot as usize + 4, pointer);
        relocations.extend([new_registry + slot, new_registry + slot + 4]);
    }
    let insert_at = 1 + Section::ALL.iter().filter(|section| sections.contains(section)).count();
    names.splice(insert_at..insert_at, extra_entries.iter().map(|name| (*name).to_owned()));
    for index in 0..padded_count {
        let name = names.get(index).unwrap_or_else(|| names.last().unwrap());
        let pointer = if let Some(&pointer) = extra_pointers.get(name.as_str()) {
            pointer
        } else {
            let index = stock_names.iter().position(|old| old == name).unwrap();
            let slot = menu_address + index as u32 * 4;
            if !original_slots.contains(&slot) {
                return device("a stock Settings pointer has no relative relocation");
            }
            elf.read(slot)?
        };
        put(&mut prefix, index * 4, pointer);
        relocations.push(prefix_address + (index * 4) as u32);
    }
    for shift in (0..registry_size).step_by(4) {
        let slot = registry.address + shift;
        if !original_slots.contains(&slot) {
            return device("a stock screen registration pointer has no relative relocation");
        }
        put(&mut prefix, registry_at + shift as usize, elf.read(slot)?);
        relocations.push(new_registry + shift);
    }

    // Copy the stock relocations into the appended read-only payload and place
    // the new entries after the stock R_ARM_RELATIVE prefix in the loader's table.
    let dyn_section = elf.section(".dynamic")?;
    let mut dynamic_tags = BTreeMap::new();
    for at in (dyn_section.offset..dyn_section.offset + dyn_section.size).step_by(8) {
        let tag = word(elf.data, at)?;
        if tag == 0 {
            break;
        }
        if matches!(tag, DT_REL | DT_RELSZ | DT_RELENT | DT_RELCOUNT)
            && dynamic_tags.insert(tag, (at + 4, word(elf.data, at + 4)?)).is_some()
        {
            return device("duplicate dynamic relocation tag");
        }
    }
    let (Some(&(rel_tag, rel_address)), Some(&(size_tag, rel_size)), Some(&(_, 8)), Some(&(count_tag, rel_count))) = (
        dynamic_tags.get(&DT_REL),
        dynamic_tags.get(&DT_RELSZ),
        dynamic_tags.get(&DT_RELENT),
        dynamic_tags.get(&DT_RELCOUNT),
    ) else {
        return device("the stock dynamic section lacks the reviewed relocation tags");
    };
    let relative_bytes = rel_count as usize * 8;
    if rel_address != rel.address
        || rel_size as usize != old_rel.len()
        || old_rel.len() % 8 != 0
        || relative_bytes > old_rel.len()
        || old_rel[..relative_bytes].chunks_exact(8).any(|item| word(item, 4).ok() != Some(R_ARM_RELATIVE))
    {
        return device("the stock relative relocation prefix does not match the loader tags");
    }
    let rel_offset = align(output.len(), 4)?;
    let payload = headers.last_mut().unwrap();
    let new_rel = payload[2] + (rel_offset as u32 - payload[1]);
    output.resize(rel_offset, 0);
    output.extend_from_slice(&old_rel[..relative_bytes]);
    for slot in &relocations {
        output.extend_from_slice(&slot.to_le_bytes());
        output.extend_from_slice(&R_ARM_RELATIVE.to_le_bytes());
    }
    output.extend_from_slice(&old_rel[relative_bytes..]);
    let new_rel_size = (output.len() - rel_offset) as u32;
    payload[4] = output.len() as u32 - payload[1];
    payload[5] = payload[4];

    redirect(elf, output, menu_build, prefix_address)?;
    redirect(elf, output, menu_lookup, prefix_address)?;
    redirect(elf, output, register, new_registry)?;
    let menu_loop = elf.offset(bound)?;
    let lookup_loop = elf.offset(lookup_bound)?;
    put(output, menu_loop, (elf.read(bound)? & !0xfff) | (names.len() as u32 * 4));
    put(output, lookup_loop, (elf.read(lookup_bound)? & !0xfff) | (padded_count as u32 - 1));

    let old_offset = old[1] as usize;
    let writable = bytes(output, old_offset, old[4] as usize)?.to_vec();
    let relocated_offset = align(output.len(), alignment)?;
    output.resize(relocated_offset, 0);
    output.extend_from_slice(&prefix);
    output.extend_from_slice(&writable);
    let new_old_start = relocated_offset + prefix_size;
    let shift = new_old_start - old_offset;
    for (at, value) in [(rel_tag, new_rel), (size_tag, new_rel_size), (count_tag, rel_count + relocations.len() as u32)]
    {
        put(output, at + shift, value);
    }
    headers[rw] = [
        PT_LOAD,
        relocated_offset as u32,
        prefix_address,
        prefix_address,
        old[4] + prefix_size as u32,
        old[5] + prefix_size as u32,
        old[6],
        old[7],
    ];
    headers[dynamic][1] += shift as u32;
    headers[relro][1] = relocated_offset as u32;
    headers[relro][2] = prefix_address;
    headers[relro][3] = prefix_address;
    headers[relro][4] += prefix_size as u32;
    headers[relro][5] += prefix_size as u32;

    let section_headers = word(elf.data, 32)? as usize;
    for (i, section) in elf.sections.iter().enumerate() {
        let at = section_headers + i * 40;
        if section.name == ".rel.dyn" {
            put(output, at + 12, new_rel);
            put(output, at + 16, rel_offset as u32);
            put(output, at + 20, new_rel_size);
        } else if section.flags & 2 != 0 && section.address >= old[2] && section.address < old[2] + old[5] {
            if section.offset < old_offset || section.offset > old_offset + old[4] as usize {
                return device("a writable section does not map through the stock segment");
            }
            put(output, at + 16, (section.offset + shift) as u32);
        }
    }

    Ok(serde_json::json!({
        "address": prefix_address,
        "bound": bound,
        "lookup_bound": lookup_bound,
        "registry_address": new_registry,
        "names": names
    }))
}
