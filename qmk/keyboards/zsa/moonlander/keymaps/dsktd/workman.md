# Workman layer (stored, not built)

Removed from the keymap on 2026-11-21: only QWERTY is used. To bring it back, add `L_WRKMN` after
`L_QWRTY` in `enum layers`, paste the block below into `keymaps[]`, put `DF(L_WRKMN)` back on a
thumb key (it was beside `DF(L_QWRTY)` on the media and number layers), and add "workman" back
to `LAYERS` in the dotfiles' moonlander-leds `usage.rs` (same position: layer numbers shift).

```c
    [L_WRKMN] = LAYOUT(
        KC_GRV,  KC_1,    KC_2,    KC_3,    KC_4,   KC_5,    KC_6,           KC_EQL,     KC_6,    KC_7,    KC_8,    KC_9,    KC_0,    _______,
        KC_TAB,  KC_Q,    KC_D,    KC_R,    KC_W,   KC_B,    MO(L_NUMB),     KC_MINS,    KC_J,    KC_F,    KC_U,    KC_P,    KC_SCLN, KC_BSLS,
        KC_LALT, KC_A,    KC_S,    KC_H,    KC_T,   KC_G,    MO(L_SYMB),     MO(L_MDIA), KC_Y,    KC_N,    KC_E,    KC_O,    KC_I,    KC_QUOT,
        KC_LSFT, KC_Z,    KC_X,    KC_M,    KC_C,   KC_V,                                KC_K,    KC_L,    KC_COMM, KC_DOT,  KC_SLSH, KC_RSFT,
        KC_LCTL, _______, _______, KC_DOWN, KC_UP,           KC_ESC,         KC_ESC,              KC_LEFT, KC_RGHT, KC_LBRC, KC_RBRC, _______,
                                            KC_SPC, KC_LGUI, KC_BSPC,        KC_LALT,    KC_RSFT, KC_ENT
    ),
```
