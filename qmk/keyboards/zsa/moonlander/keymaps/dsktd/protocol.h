#pragma once

#include <stdbool.h>
#include <stdint.h>

// Raw HID protocol: the first byte of a packet is the event. The host mirror
// of this layout is the `moonlander-leds` daemon (dotfiles, pkgs/moonlander-leds).
enum event {
    event_NONE          = 0,
    event_MEDIA         = 1,  // unused, superseded by event_MEDIA_KEYS
    event_WORKSPACE     = 2,  // data[1]: bitmask, MSB is workspace 0
    event_VOLUME        = 3,  // data[1]: volume; sent to the host as [3, volume, MAX_VOLUME]
    event_WORKSPACES    = 4,  // data[1..8]: workspace_state of workspaces 0..7, | WORKSPACE_DIM
    event_PALETTE       = 5,  // data[1..15]: RGB per workspace_state, data[16]: dim factor (255 = full brightness)
    event_MEDIA_KEYS    = 6,  // data[1..9]: RGB of the previous, play/pause and next keys
    event_WIDTH         = 7,  // data[1..6]: brightness (0-255) of the 1-6 keys, window coverage of that sixth of the output
    event_BALANCE       = 8,  // data[1]: game/chat balance, 0 all game, 255 all chat; sent to the host as [9, key, BALANCE_KEY_COUNT] with event_BALANCE_SET
    event_CAST          = 10, // data[1]: 1 while the screen is shared, 0 otherwise
    event_LOCK          = 11, // data[1]: 1 while the desktop is locked, 0 otherwise
    event_USAGE_REQUEST = 12, // host to keyboard: data[1] is the chunk to read and reset
    event_USAGE         = 13, // keyboard to host: data[1] chunk, data[2..31] USAGE_PER_CHUNK little-endian uint16 counts
    event_MOD_REQUEST   = 14, // host to keyboard: data[1] is the chunk of modifier-combo counters to read and reset
    event_MOD           = 15, // keyboard to host: like event_USAGE, counts per (modifier mask, row, col)
    event_DUAL_REQUEST  = 16, // host to keyboard: data[1] is the chunk of tap/hold counters to read and reset
    event_DUAL          = 17, // keyboard to host: like event_USAGE, counts per (dual_kind, row, col)
    event_LAYER_REQUEST = 18, // host to keyboard: data[1] is the chunk of layer counters to read and reset
    event_LAYER         = 19, // keyboard to host: like event_USAGE, entries then held time (100 ms units) per layer
    event_BALANCE_SET   = 9,  // keyboard to host: data[1] is the pressed balance key, 0 (all game) to BALANCE_KEY_COUNT - 1 (all chat)
};

#define WORKSPACE_COUNT 8

// High bit of a slot byte: the workspace is on a display that is not focused.
#define WORKSPACE_DIM 0x80

enum workspace_state {
    workspace_EMPTY    = 0, // no windows: LED off
    workspace_OCCUPIED = 1,
    workspace_FOCUSED  = 2,
    workspace_URGENT   = 3,
    workspace_VISIBLE  = 4, // active on a display but not focused
};

// Decode an event_WORKSPACES packet into states and dim flags. Unknown states
// read as empty.
static inline bool decode_workspaces(const uint8_t *data, uint8_t length, uint8_t states[WORKSPACE_COUNT], bool dim[WORKSPACE_COUNT]) {
    if (length < 1 + WORKSPACE_COUNT || data[0] != event_WORKSPACES) {
        return false;
    }
    for (uint8_t i = 0; i < WORKSPACE_COUNT; i++) {
        uint8_t state = data[1 + i] & ~WORKSPACE_DIM;
        states[i]     = state <= workspace_VISIBLE ? state : workspace_EMPTY;
        dim[i]        = data[1 + i] & WORKSPACE_DIM;
    }
    return true;
}

#define PALETTE_DIM_INDEX (1 + 3 * (workspace_VISIBLE + 1))

// Decode an event_PALETTE packet into a colour per workspace_state and the dim
// factor.
static inline bool decode_palette(const uint8_t *data, uint8_t length, uint8_t colors[][3], uint8_t *dim) {
    if (length < PALETTE_DIM_INDEX + 1 || data[0] != event_PALETTE) {
        return false;
    }
    for (uint8_t state = 0; state <= workspace_VISIBLE; state++) {
        for (uint8_t c = 0; c < 3; c++) {
            colors[state][c] = data[1 + 3 * state + c];
        }
    }
    *dim = data[PALETTE_DIM_INDEX];
    return true;
}

#define MEDIA_KEY_COUNT 3

// Decode an event_MEDIA_KEYS packet into a colour per media key, in the order
// previous, play/pause, next.
static inline bool decode_media_keys(const uint8_t *data, uint8_t length, uint8_t colors[MEDIA_KEY_COUNT][3]) {
    if (length < 1 + 3 * MEDIA_KEY_COUNT || data[0] != event_MEDIA_KEYS) {
        return false;
    }
    for (uint8_t key = 0; key < MEDIA_KEY_COUNT; key++) {
        for (uint8_t c = 0; c < 3; c++) {
            colors[key][c] = data[1 + 3 * key + c];
        }
    }
    return true;
}

#define WIDTH_KEY_COUNT 6

// Decode an event_WIDTH packet into a brightness per number key (1 to 6).
static inline bool decode_width(const uint8_t *data, uint8_t length, uint8_t levels[WIDTH_KEY_COUNT]) {
    if (length < 1 + WIDTH_KEY_COUNT || data[0] != event_WIDTH) {
        return false;
    }
    for (uint8_t key = 0; key < WIDTH_KEY_COUNT; key++) {
        levels[key] = data[1 + key];
    }
    return true;
}

#define BALANCE_KEY_COUNT 7

// Decode an event_BALANCE packet into the nearest balance key.
static inline bool decode_balance(const uint8_t *data, uint8_t length, uint8_t *key) {
    if (length < 2 || data[0] != event_BALANCE) {
        return false;
    }
    *key = (data[1] * (BALANCE_KEY_COUNT - 1) + 127) / 255;
    return true;
}

// Decode an on/off packet (event_CAST, event_LOCK) into its on/off flag.
static inline bool decode_flag(const uint8_t *data, uint8_t length, uint8_t event, bool *on) {
    if (length < 2 || data[0] != event) {
        return false;
    }
    *on = data[1] != 0;
    return true;
}

// Key-press counters per (layer, row, col), flattened as (layer * rows + row) * cols + col and
// sent in chunks of USAGE_PER_CHUNK counts. Only the position is counted, never keycodes.
#define USAGE_PER_CHUNK 15

static inline uint16_t usage_chunks(uint16_t total) {
    return (total + USAGE_PER_CHUNK - 1) / USAGE_PER_CHUNK;
}

// Encode chunk `chunk` of `counts` into a packet of 32 bytes tagged `event`, and zero those
// counts. Positions past the end are sent as zero. False when the chunk is out of range.
static inline bool encode_counts(uint8_t event, uint16_t *counts, uint16_t total, uint8_t chunk, uint8_t packet[32]) {
    if (chunk >= usage_chunks(total)) {
        return false;
    }
    packet[0] = event;
    packet[1] = chunk;
    for (uint8_t i = 0; i < USAGE_PER_CHUNK; i++) {
        uint16_t index = chunk * USAGE_PER_CHUNK + i;
        uint16_t count = index < total ? counts[index] : 0;
        if (index < total) {
            counts[index] = 0;
        }
        packet[2 + 2 * i]     = count & 0xFF;
        packet[2 + 2 * i + 1] = count >> 8;
    }
    return true;
}

static inline bool encode_usage(uint16_t *counts, uint16_t total, uint8_t chunk, uint8_t packet[32]) {
    return encode_counts(event_USAGE, counts, total, chunk, packet);
}

// Presses made while a modifier is held, per (mask, row, col) flattened as
// (mask * rows + row) * cols + col. The mask folds left and right modifiers: 1 ctrl, 2 shift,
// 4 alt, 8 gui. Mask 0 (no modifier) is never counted; it is in the plain usage counters.
#define MOD_MASKS 16

static inline uint8_t mod_mask(uint8_t mods) {
    return (mods | mods >> 4) & 0x0F;
}

// What a dual-role key (mod-tap, layer-tap) did when pressed. A misfire is a hold released
// within DUAL_MISFIRE_MS of resolving, or a tap repeated within DUAL_MISFIRE_MS of the last tap.
enum dual_kind {
    dual_TAP     = 0,
    dual_HOLD    = 1,
    dual_MISFIRE = 2,
    dual_KINDS   = 3,
};

#define DUAL_MISFIRE_MS 150

// Layer counters: [0..layers) entries into the layer, [layers..2*layers) time spent with it the
// highest active layer, in 100 ms units.
#define LAYER_TIME_UNIT_MS 100
