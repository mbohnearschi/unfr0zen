#ifndef UNFR0ZEN_ROOM_SENSOR_NAMES_H
#define UNFR0ZEN_ROOM_SENSOR_NAMES_H

/* Bounds of nestlocal's room-sensors.catalog (sensor ID, tab, display name). */
#define ROOM_SENSOR_COUNT     6
#define ROOM_SENSOR_ID_SIZE   44
#define ROOM_SENSOR_NAME_SIZE 97
#define ROOM_SENSOR_NAME_MAX  24
#define ROOM_CATALOG_SIZE     (ROOM_SENSOR_COUNT * (ROOM_SENSOR_ID_SIZE + ROOM_SENSOR_NAME_SIZE) + 1)
#define ROOM_NAME_TOKEN_SIZE  8

struct room_sensor_row {
    char id[ROOM_SENSOR_ID_SIZE];
    char name[ROOM_SENSOR_NAME_SIZE];
};

struct room_sensor_catalog {
    char text[ROOM_CATALOG_SIZE];
    struct room_sensor_row rows[ROOM_SENSOR_COUNT];
    int count;
};

static unsigned int room_text_length(const char *text) {
    unsigned int length = 0;
    while (text[length])
        length++;
    return length;
}

static void room_copy(char *to, const char *from, unsigned int length) {
    for (unsigned int i = 0; i < length; i++)
        to[i] = from[i];
    to[length] = 0;
}

static void room_parse_catalog(struct room_sensor_catalog *catalog) {
    const char *line = catalog->text;
    catalog->count = 0;
    while (*line && catalog->count < ROOM_SENSOR_COUNT) {
        const char *split = line, *end;
        while (*split && *split != '\t' && *split != '\n')
            split++;
        end = split;
        while (*end && *end != '\n')
            end++;
        if (*split == '\t' && split > line && split - line < ROOM_SENSOR_ID_SIZE && end > split + 1 &&
            end - split <= ROOM_SENSOR_NAME_SIZE) {
            struct room_sensor_row *row = &catalog->rows[catalog->count++];
            room_copy(row->id, line, split - line);
            room_copy(row->name, split + 1, end - split - 1);
        }
        line = end + (*end != 0);
    }
}

static inline int room_name_append(char *name, char key) {
    unsigned int length = 0, characters = 0;
    while (name[length]) {
        if (((unsigned char)name[length] & 0xc0) != 0x80)
            characters++;
        length++;
    }
    if (characters >= ROOM_SENSOR_NAME_MAX || length + 1 >= ROOM_SENSOR_NAME_SIZE)
        return 0;
    name[length] = key;
    name[length + 1] = 0;
    return 1;
}

static inline void room_name_delete(char *name) {
    unsigned int length = room_text_length(name);
    if (!length)
        return;
    do {
        length--;
    } while (length && ((unsigned char)name[length] & 0xc0) == 0x80);
    name[length] = 0;
}

static void room_name_token(char *out, unsigned int token) {
    for (int i = 0; i < ROOM_NAME_TOKEN_SIZE; i++)
        out[i] = "0123456789abcdef"[(token >> (28 - 4 * i)) & 15];
    out[ROOM_NAME_TOKEN_SIZE] = 0;
}

static const char *room_name_reply(const char *status, const char *token) {
    for (int i = 0; i < ROOM_NAME_TOKEN_SIZE; i++)
        if (status[i] != token[i])
            return 0;
    return status[ROOM_NAME_TOKEN_SIZE] == '\t' ? status + ROOM_NAME_TOKEN_SIZE + 1 : 0;
}

#endif
