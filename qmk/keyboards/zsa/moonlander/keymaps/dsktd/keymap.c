#include QMK_KEYBOARD_H
#include "raw_hid.h"
#include "protocol.h"
#include "usb_device_state.h"

#define ALPHA_KEYCOLOR RGB_PURPLE
#define WS_KEYCOLOR RGB_RED // legacy event_WORKSPACE bitmask
// Per-workspace states (event_WORKSPACES); an empty workspace is off.
#define WS_OCCUPIED_KEYCOLOR RGB_BLUE
#define WS_FOCUSED_KEYCOLOR RGB_WHITE
#define WS_URGENT_KEYCOLOR RGB_RED
#define WS_VISIBLE_KEYCOLOR 0x30, 0x30, 0x30 // dimmed focused colour
#define WS_DIM_FACTOR 128                    // of 255, for an unfocused display
// These are the defaults until the host sends event_PALETTE.
#define MOD_KEYCOLOR RGB_SPRINGGREEN
#define LAYER_KEYCOLOR RGB_SPRINGGREEN

// Window coverage of the output (event_WIDTH) on the number keys; it fades out when the
// host stops sending for WIDTH_TIMEOUT_MS or the USB host goes away.
#define WIDTH_KEYCOLOR 0xFF, 0xA0, 0x00 // amber: unused by the other states
#define WIDTH_TIMEOUT_MS 5000
// Every host overlay (workspaces, media, palette, width) is dropped after this
// long without any packet, leaving the plain keymap lighting.
#define HOST_TIMEOUT_MS 5000
#define WIDTH_FADE_MS 1000

// Game/chat balance strip (event_BALANCE) on the media layer's top row: the current step is
// shown at full brightness, the others dimmed, blending from game to chat.
#define BALANCE_GAME_KEYCOLOR 0x00, 0x80, 0xFF
#define BALANCE_CHAT_KEYCOLOR 0xFF, 0x40, 0x00
#define BALANCE_DIM_FACTOR 40 // of 255

// While the screen is shared (event_CAST) the top-left key strobes, on every layer.
#define CAST_KEYCOLOR 0xFF, 0x00, 0x00
#define CAST_KEY_LED 0     // matrix [0,0], KC_GRV
#define CAST_STROBE_MS 125 // half a period: 4 Hz
// While the desktop is locked (event_LOCK) the board breathes instead of its normal lighting.
#define LOCK_BREATH_MS 4000
#define LOCK_MAX_BRIGHTNESS 40 // of 255

#define MAX_VOLUME 5

enum layers {
    L_QWRTY,
    L_SYMB,
    L_NUMB,
    L_MDIA,
    L_COUNT,
};

enum custom_keycodes {
    VOL_1 = SAFE_RANGE,
    VOL_2,
    VOL_3,
    VOL_4,
    VOL_5,
    BAL_1,
    BAL_2,
    BAL_3,
    BAL_4,
    BAL_5,
    BAL_6,
    BAL_7,
};

// clang-format off
const uint16_t PROGMEM keymaps[][MATRIX_ROWS][MATRIX_COLS] = {
    [L_QWRTY] = LAYOUT(
        KC_GRV,  KC_1,    KC_2,    KC_3,    KC_4,   KC_5,    KC_6,           KC_EQL,     KC_6,   KC_7,    KC_8,    KC_9,    KC_0,    KC_MINS,
        KC_TAB,  KC_Q,    KC_W,    KC_E,    KC_R,   KC_T,    KC_Y,           MO(L_NUMB), KC_Y,   KC_U,    KC_I,    KC_O,    KC_P,    KC_BSLS,
        KC_LALT, KC_A,    KC_S,    KC_D,    KC_F,   KC_G,    MO(L_SYMB),     MO(L_MDIA), KC_H,   KC_J,    KC_K,    KC_L,    KC_SCLN, KC_QUOT,
        KC_LSFT, KC_Z,    KC_X,    KC_C,    KC_V,   KC_B,                                KC_N,   KC_M,    KC_COMM, KC_DOT,  KC_SLSH, KC_RSFT,
        KC_LCTL, _______, _______, KC_DOWN, KC_UP,           KC_ESC,         KC_ESC,             KC_LEFT, KC_RGHT, KC_LBRC, KC_RBRC, KC_LCTL,
                                            KC_SPC, KC_LGUI, KC_BSPC,        KC_LALT,    KC_ESC, KC_ENT
    ),

    [L_SYMB] = LAYOUT(
        KC_ESC,  KC_F1,   KC_F2,   KC_F3,   KC_F4,   KC_F5,   KC_F6,       KC_F5,   KC_F6,   KC_F7,   KC_F8,   KC_F9,   KC_F10,  KC_F11,
        _______, _______, _______, _______, _______, _______, _______,     _______, KC_LPRN, KC_RPRN, KC_LBRC, KC_RBRC, _______, KC_F12,
        _______, _______, _______, _______, _______, _______, _______,     _______, KC_LEFT, KC_DOWN, KC_UP,   KC_RGHT, _______, KC_PGUP,
        _______, _______, _______, _______, _______, _______,                       KC_MINS, KC_PLUS, KC_LCBR, KC_RCBR, _______, KC_PGDN,
        _______, _______, _______, _______, _______,          RM_NEXT,     RM_TOGG,          KC_EQL,  _______, _______, _______, _______,
                                            RM_VALD, RM_VALU, KC_DEL,      _______, RM_HUED, KC_DEL
    ),

    [L_MDIA] = LAYOUT(
        _______, _______, _______, _______, _______,     _______,     _______,     BAL_1,   BAL_2,   BAL_3,   BAL_4, BAL_5,   BAL_6,   BAL_7,
        _______, MS_BTN1, MS_UP,   MS_BTN2, _______,     _______,     _______,     KC_MPRV, KC_MPLY, KC_MNXT, VOL_4, _______, _______, QK_BOOT,
        _______, MS_LEFT, MS_DOWN, MS_RGHT, KC_PSCR,     _______,     _______,     _______, _______, _______, VOL_3, _______, _______, _______,
        _______, _______, _______, _______, _______,     _______,                           _______, _______, VOL_2, _______, _______, _______,
        _______, _______, _______, _______, _______,                  _______,     _______,          _______, VOL_1, _______, _______, _______,
                                            _______, _______, _______,     _______, _______, _______
    ),

    [L_NUMB] = LAYOUT(
        _______, _______, _______, _______, _______,     _______,     _______,     _______, _______, _______, _______, _______, _______, _______,
        _______, _______, _______, _______, _______,     _______,     _______,     _______, KC_P7,   KC_P8,   KC_P9,   KC_PPLS, _______, _______,
        _______, _______, _______, _______, _______,     _______,     _______,     _______, KC_P4,   KC_P5,   KC_P6,   KC_PMNS, _______, _______,
        _______, _______, _______, _______, _______,     _______,                           KC_P1,   KC_P2,   KC_P3,   KC_PAST, _______, _______,
        _______, _______, _______, _______, _______,                  _______,     _______,          KC_P0,   KC_PDOT, KC_PSLS, _______, _______,
                                            _______, _______, _______,     _______, _______, _______
    ),
};
// clang-format on

// LED indices (matrix positions) for the lighting below.
// clang-format off
static const uint8_t alpha_keys[] = {
     6, 11, 16, 21, 26,     62, 57, 52, 47, 42,
     7, 12, 17, 22, 27,     63, 58, 53, 48, 43,
     8, 13, 18, 23, 28,     64, 59, 54, 49, 44,
};
// Base layer: the left half's alphas stay lit (its inner ones are the workspace keys, dark
// while empty). The right half mirrors that: its inner keys (the mirror of 6,11,16,21,7,12,17,22)
// are off and only the bottom row and the outermost column are lit.
static const uint8_t base_alpha_keys[] = {
    26, 27, 28,   64, 59, 54, 49, 42, 43, 44,
     8, 13, 18, 23,
     6, 11, 16, 21,
     7, 12, 17, 22,
};
static const uint8_t numpad_keys[] = {
    62, 57, 52,
    63, 58, 53,
    64, 59, 54,
        60,
};
// clang-format on
static const uint8_t mod_keys[]   = {2, 3, 4, 70};
static const uint8_t layer_keys[] = {31, 67};
// Previous, play/pause, next (event_MEDIA_KEYS order).
static const uint8_t media_keys[MEDIA_KEY_COUNT] = {66, 62, 57};
static const uint8_t workspace_keys[]            = {6, 11, 16, 21, 7, 12, 17, 22};
// Lowest to highest volume step.
static const uint8_t volume_keys[] = {55, 54, 53, 52, 51};

static bool    media_keys_seen = false;
static rgb_t   media_colors[MEDIA_KEY_COUNT];
static uint8_t workspace_mask        = 0;
static bool    workspace_states_seen = false;
static uint8_t workspace_states[WORKSPACE_COUNT];
static bool    workspace_dim[WORKSPACE_COUNT];
static rgb_t   workspace_colors[] = {
    [workspace_EMPTY] = {0, 0, 0}, [workspace_OCCUPIED] = {WS_OCCUPIED_KEYCOLOR}, [workspace_FOCUSED] = {WS_FOCUSED_KEYCOLOR}, [workspace_URGENT] = {WS_URGENT_KEYCOLOR}, [workspace_VISIBLE] = {WS_VISIBLE_KEYCOLOR},
};
static const rgb_t workspace_default_colors[] = {
    [workspace_EMPTY] = {0, 0, 0}, [workspace_OCCUPIED] = {WS_OCCUPIED_KEYCOLOR}, [workspace_FOCUSED] = {WS_FOCUSED_KEYCOLOR}, [workspace_URGENT] = {WS_URGENT_KEYCOLOR}, [workspace_VISIBLE] = {WS_VISIBLE_KEYCOLOR},
};
static uint8_t workspace_dim_factor = WS_DIM_FACTOR;
static uint8_t current_volume       = 0;
// LED indices of the right half's top row, from the split outwards (matrix [6,0] to [6,6]).
static const uint8_t balance_keys[BALANCE_KEY_COUNT] = {65, 61, 56, 51, 46, 41, 36};
static uint8_t       balance_key                     = 0;
static bool          balance_seen                    = false;
// Presses per layer and matrix position since the host last read them (saturating).
#define USAGE_TOTAL (L_COUNT * MATRIX_ROWS * MATRIX_COLS)
static uint16_t usage_counts[USAGE_TOTAL];
// Presses while a modifier is held, per modifier mask and matrix position (saturating).
static uint16_t mod_counts[MOD_MASKS * MATRIX_ROWS * MATRIX_COLS];
// Taps, holds and misfires of dual-role keys per matrix position, by dual_kind (saturating).
static uint16_t dual_counts[dual_KINDS * MATRIX_ROWS * MATRIX_COLS];
static uint16_t dual_last_tap[MATRIX_ROWS][MATRIX_COLS];
static uint16_t dual_hold_start[MATRIX_ROWS][MATRIX_COLS];
static bool     dual_holding[MATRIX_ROWS][MATRIX_COLS];
// Entries into each layer, then time with it the highest layer in 100 ms units (saturating).
#define LAYER_COUNT L_COUNT
static uint16_t layer_counts[2 * LAYER_COUNT];
static uint8_t  layer_now   = 0;
static uint16_t layer_since = 0;
static bool     cast_active = false;
static bool     lock_active = false;
// LED indices of the left half's top row, columns 1-6: the 1-5 keys and the inner 6 key.
static const uint8_t width_keys[WIDTH_KEY_COUNT] = {5, 10, 15, 20, 25, 29};
static uint8_t       width_levels[WIDTH_KEY_COUNT];
static uint32_t      width_received = 0;
static bool          width_seen     = false;
static uint32_t      host_received  = 0;
static bool          host_active    = false;

static void set_keys(const uint8_t *keys, uint8_t count, uint8_t red, uint8_t green, uint8_t blue) {
    for (uint8_t i = 0; i < count; i++) {
        rgb_matrix_set_color(keys[i], red, green, blue);
    }
}
#define SET_KEYS(keys, ...) set_keys(keys, ARRAY_SIZE(keys), __VA_ARGS__)

static void send_balance(uint8_t key) {
    uint8_t buffer[32] = {event_BALANCE_SET, key, BALANCE_KEY_COUNT};
    raw_hid_send(buffer, sizeof(buffer));
}

static void send_volume(uint8_t volume) {
    uint8_t buffer[32] = {event_VOLUME, volume, MAX_VOLUME};
    raw_hid_send(buffer, sizeof(buffer));
}

static void count_up(uint16_t *counter, uint16_t by) {
    *counter = *counter > UINT16_MAX - by ? UINT16_MAX : *counter + by;
}

// Credit the time since the last change to the layer that was on top.
static void layer_account(void) {
    uint16_t now = timer_read();
    count_up(&layer_counts[LAYER_COUNT + layer_now], (uint16_t)(now - layer_since) / LAYER_TIME_UNIT_MS);
    // Keep the remainder: advance by whole units only.
    layer_since += (uint16_t)(now - layer_since) / LAYER_TIME_UNIT_MS * LAYER_TIME_UNIT_MS;
}

static void layer_changed(layer_state_t state, layer_state_t default_state) {
    uint8_t layer = get_highest_layer(state | default_state);
    if (layer != layer_now && layer < LAYER_COUNT) {
        layer_account();
        layer_now = layer;
        count_up(&layer_counts[layer], 1);
    }
}

layer_state_t layer_state_set_user(layer_state_t state) {
    layer_changed(state, default_layer_state);
    return state;
}

layer_state_t default_layer_state_set_user(layer_state_t state) {
    layer_changed(layer_state, state);
    return state;
}

// Count taps, holds and misfires of mod-tap and layer-tap keys.
static void count_dual(keyrecord_t *record) {
    uint8_t  row = record->event.key.row, col = record->event.key.col;
    uint16_t time = record->event.time;
    uint16_t base = row * MATRIX_COLS + col;
    if (record->event.pressed) {
        if (record->tap.count > 0) {
            bool again = dual_last_tap[row][col] && (uint16_t)(time - dual_last_tap[row][col]) < DUAL_MISFIRE_MS;
            count_up(&dual_counts[(again ? dual_MISFIRE : dual_TAP) * MATRIX_ROWS * MATRIX_COLS + base], 1);
            dual_last_tap[row][col] = time ? time : 1;
        } else {
            count_up(&dual_counts[dual_HOLD * MATRIX_ROWS * MATRIX_COLS + base], 1);
            dual_hold_start[row][col] = time;
            dual_holding[row][col]    = true;
        }
    } else if (dual_holding[row][col]) {
        dual_holding[row][col] = false;
        // The press time is the original one, so a hold lasts at least TAPPING_TERM.
        if ((uint16_t)(time - dual_hold_start[row][col]) < TAPPING_TERM + DUAL_MISFIRE_MS) {
            count_up(&dual_counts[dual_MISFIRE * MATRIX_ROWS * MATRIX_COLS + base], 1);
        }
    }
}

void raw_hid_receive(uint8_t *data, uint8_t length) {
    host_received = timer_read32();
    host_active   = true;
    switch (data[0]) {
        case event_MEDIA_KEYS: {
            uint8_t colors[MEDIA_KEY_COUNT][3];
            if (decode_media_keys(data, length, colors)) {
                for (uint8_t key = 0; key < MEDIA_KEY_COUNT; key++) {
                    media_colors[key] = (rgb_t){colors[key][0], colors[key][1], colors[key][2]};
                }
                media_keys_seen = true;
            }
            break;
        }

        case event_WORKSPACE:
            workspace_mask = data[1];
            break;

        case event_WORKSPACES:
            workspace_states_seen = decode_workspaces(data, length, workspace_states, workspace_dim) || workspace_states_seen;
            break;

        case event_PALETTE: {
            uint8_t colors[workspace_VISIBLE + 1][3];
            uint8_t dim;
            if (decode_palette(data, length, colors, &dim)) {
                for (uint8_t state = 0; state <= workspace_VISIBLE; state++) {
                    workspace_colors[state] = (rgb_t){colors[state][0], colors[state][1], colors[state][2]};
                }
                workspace_dim_factor = dim;
            }
            break;
        }

        case event_WIDTH:
            if (decode_width(data, length, width_levels)) {
                width_received = timer_read32();
                width_seen     = true;
            }
            break;

        case event_BALANCE:
            balance_seen = decode_balance(data, length, &balance_key) || balance_seen;
            break;

        case event_USAGE_REQUEST: {
            uint8_t packet[32] = {0};
            if (encode_usage(usage_counts, USAGE_TOTAL, data[1], packet)) {
                raw_hid_send(packet, sizeof(packet));
            }
            break;
        }

        case event_MOD_REQUEST: {
            uint8_t packet[32] = {0};
            if (encode_counts(event_MOD, mod_counts, sizeof(mod_counts) / sizeof(mod_counts[0]), data[1], packet)) {
                raw_hid_send(packet, sizeof(packet));
            }
            break;
        }

        case event_DUAL_REQUEST: {
            uint8_t packet[32] = {0};
            if (encode_counts(event_DUAL, dual_counts, sizeof(dual_counts) / sizeof(dual_counts[0]), data[1], packet)) {
                raw_hid_send(packet, sizeof(packet));
            }
            break;
        }

        case event_LAYER_REQUEST: {
            uint8_t packet[32] = {0};
            layer_account();
            if (encode_counts(event_LAYER, layer_counts, sizeof(layer_counts) / sizeof(layer_counts[0]), data[1], packet)) {
                raw_hid_send(packet, sizeof(packet));
            }
            break;
        }

        case event_CAST:
            decode_flag(data, length, event_CAST, &cast_active);
            break;

        case event_LOCK:
            decode_flag(data, length, event_LOCK, &lock_active);
            break;

        case event_VOLUME:
            current_volume = MIN(data[1], MAX_VOLUME);
            break;
    }
}

// Forget everything the host drove, back to the keymap's own lighting.
static void host_release(void) {
    host_active           = false;
    media_keys_seen       = false;
    workspace_mask        = 0;
    workspace_states_seen = false;
    width_seen            = false;
    balance_seen          = false;
    cast_active           = false;
    lock_active           = false;
    memcpy(workspace_colors, workspace_default_colors, sizeof(workspace_colors));
    workspace_dim_factor = WS_DIM_FACTOR;
    current_volume       = 0;
}

static bool base_indicators(void) {
    rgb_matrix_set_color_all(0, 0, 0);
    switch (get_highest_layer(layer_state)) {
        case L_QWRTY:
            SET_KEYS(base_alpha_keys, ALPHA_KEYCOLOR);
            for (uint8_t ws = 0; ws < ARRAY_SIZE(workspace_keys); ws++) {
                if (workspace_states_seen) {
                    rgb_t color = workspace_colors[workspace_states[ws]];
                    if (workspace_dim[ws]) {
                        color = (rgb_t){color.r * workspace_dim_factor / 255, color.g * workspace_dim_factor / 255, color.b * workspace_dim_factor / 255};
                    }
                    rgb_matrix_set_color(workspace_keys[ws], color.r, color.g, color.b);
                } else if (workspace_mask & (0x80 >> ws)) {
                    rgb_matrix_set_color(workspace_keys[ws], WS_KEYCOLOR);
                } else {
                    rgb_matrix_set_color(workspace_keys[ws], ALPHA_KEYCOLOR);
                }
            }
            SET_KEYS(mod_keys, MOD_KEYCOLOR);
            SET_KEYS(layer_keys, LAYER_KEYCOLOR);
            // Same media-state colours as on the media layer, only once the host has reported.
            if (media_keys_seen) {
                for (uint8_t key = 0; key < MEDIA_KEY_COUNT; key++) {
                    rgb_matrix_set_color(media_keys[key], media_colors[key].r, media_colors[key].g, media_colors[key].b);
                }
            }
            return true;
        case L_SYMB:
            SET_KEYS(alpha_keys, MOD_KEYCOLOR);
            return true;
        case L_NUMB:
            SET_KEYS(numpad_keys, ALPHA_KEYCOLOR);
            rgb_matrix_set_color(55, MOD_KEYCOLOR);
            return true;
        case L_MDIA:
            for (uint8_t key = 0; key < MEDIA_KEY_COUNT; key++) {
                rgb_t color = media_keys_seen ? media_colors[key] : (rgb_t){MOD_KEYCOLOR};
                rgb_matrix_set_color(media_keys[key], color.r, color.g, color.b);
            }
            set_keys(volume_keys, current_volume, MOD_KEYCOLOR);
            if (balance_seen) {
                for (uint8_t key = 0; key < BALANCE_KEY_COUNT; key++) {
                    // Blend from game (key 0) to chat (last key) in steps of 1/6.
                    uint8_t     step   = BALANCE_KEY_COUNT - 1;
                    const rgb_t game   = {BALANCE_GAME_KEYCOLOR};
                    const rgb_t chat   = {BALANCE_CHAT_KEYCOLOR};
                    uint16_t    bright = key == balance_key ? 255 : BALANCE_DIM_FACTOR;
                    rgb_matrix_set_color(balance_keys[key], (game.r * (step - key) + chat.r * key) / step * bright / 255, (game.g * (step - key) + chat.g * key) / step * bright / 255, (game.b * (step - key) + chat.b * key) / step * bright / 255);
                }
            }
            return true;
    }
    return false;
}

// Brightness multiplier (0-255) of the width keys: full until the host has been
// quiet for WIDTH_TIMEOUT_MS, then a linear fade over WIDTH_FADE_MS.
static uint8_t width_fade(void) {
    if (!width_seen || usb_device_state_get_configure_state() != USB_DEVICE_STATE_CONFIGURED) {
        return 0;
    }
    uint32_t quiet = timer_elapsed32(width_received);
    if (quiet <= WIDTH_TIMEOUT_MS) {
        return 255;
    }
    return quiet >= WIDTH_TIMEOUT_MS + WIDTH_FADE_MS ? 0 : 255 - 255 * (quiet - WIDTH_TIMEOUT_MS) / WIDTH_FADE_MS;
}

bool rgb_matrix_indicators_user(void) {
    if (host_active && (usb_device_state_get_configure_state() != USB_DEVICE_STATE_CONFIGURED || timer_elapsed32(host_received) > HOST_TIMEOUT_MS)) {
        host_release();
    }
    bool base = base_indicators();
    if (lock_active) {
        // Triangle wave over LOCK_BREATH_MS, in the alpha colour.
        uint32_t    phase = timer_read32() % LOCK_BREATH_MS;
        uint32_t    wave  = phase < LOCK_BREATH_MS / 2 ? phase : LOCK_BREATH_MS - phase;
        uint8_t     level = LOCK_MAX_BRIGHTNESS * wave / (LOCK_BREATH_MS / 2);
        const rgb_t col   = {ALPHA_KEYCOLOR};
        rgb_matrix_set_color_all(col.r * level / 255, col.g * level / 255, col.b * level / 255);
        base = true;
    }
    if (cast_active) {
        const rgb_t color = {CAST_KEYCOLOR};
        bool        on    = (timer_read32() / CAST_STROBE_MS) % 2 == 0;
        rgb_matrix_set_color(CAST_KEY_LED, on ? color.r : 0, on ? color.g : 0, on ? color.b : 0);
    }
    if (uint8_t fade = lock_active ? 0 : width_fade()) {
        const rgb_t color = {WIDTH_KEYCOLOR};
        for (uint8_t key = 0; key < WIDTH_KEY_COUNT; key++) {
            uint16_t level = width_levels[key] * fade / 255;
            rgb_matrix_set_color(width_keys[key], color.r * level / 255, color.g * level / 255, color.b * level / 255);
        }
    }
    return base;
}

bool process_record_user(uint16_t keycode, keyrecord_t *record) {
    if (IS_KEYEVENT(record->event) && (IS_QK_MOD_TAP(keycode) || IS_QK_LAYER_TAP(keycode))) {
        count_dual(record);
    }
    if (record->event.pressed && IS_KEYEVENT(record->event)) {
        uint8_t  layer = layer_switch_get_layer(record->event.key);
        uint16_t index = (layer * MATRIX_ROWS + record->event.key.row) * MATRIX_COLS + record->event.key.col;
        if (layer < L_COUNT && usage_counts[index] < UINT16_MAX) {
            usage_counts[index]++;
        }
        uint8_t mask = mod_mask(get_mods());
        if (mask) {
            count_up(&mod_counts[(mask * MATRIX_ROWS + record->event.key.row) * MATRIX_COLS + record->event.key.col], 1);
        }
    }
    switch (keycode) {
        case BAL_1 ... BAL_7:
            if (record->event.pressed) {
                send_balance(keycode - BAL_1);
            }
            return false;

        case VOL_1 ... VOL_5:
            if (record->event.pressed) {
                send_volume(keycode - VOL_1 + 1);
                return false;
            }
            break;
    }
    return true;
}
