// gcc -I.. -o /tmp/protocol_test protocol_test.c && /tmp/protocol_test
#include <assert.h>
#include <string.h>
#include "protocol.h"

int main(void) {
    // Golden vector, shared with the host daemon's tests.
    const uint8_t packet[32] = {4, 0, 1, 2, 3, 0x84, 0x80, 0x81, 0x83};
    uint8_t       states[WORKSPACE_COUNT];
    bool          dim[WORKSPACE_COUNT];
    assert(decode_workspaces(packet, sizeof(packet), states, dim));
    const uint8_t want[WORKSPACE_COUNT] = {0, 1, 2, 3, 4, 0, 1, 3};
    assert(memcmp(states, want, sizeof(want)) == 0);
    const bool want_dim[WORKSPACE_COUNT] = {false, false, false, false, true, true, true, true};
    assert(memcmp(dim, want_dim, sizeof(want_dim)) == 0);

    const uint8_t bad[32] = {4, 9, 0x89, 5};
    assert(decode_workspaces(bad, sizeof(bad), states, dim) && states[0] == 0 && states[1] == 0 && states[2] == 0 && states[3] == 0);
    assert(!decode_workspaces(packet, 8, states, dim));
    const uint8_t other[32] = {2};
    assert(!decode_workspaces(other, sizeof(other), states, dim));

    // Golden vector, shared with the host daemon's tests.
    const uint8_t palette[32] = {5, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 128};
    uint8_t       colors[workspace_VISIBLE + 1][3];
    uint8_t       dim_factor;
    assert(decode_palette(palette, sizeof(palette), colors, &dim_factor));
    for (uint8_t i = 0; i < 15; i++) {
        assert(colors[i / 3][i % 3] == i + 1);
    }
    assert(dim_factor == 128);
    assert(!decode_palette(palette, 16, colors, &dim_factor));
    assert(!decode_palette(packet, sizeof(packet), colors, &dim_factor));

    // Golden vector, shared with the host daemon's tests.
    const uint8_t media[32] = {6, 1, 2, 3, 4, 5, 6, 7, 8, 9};
    uint8_t       media_colors[MEDIA_KEY_COUNT][3];
    assert(decode_media_keys(media, sizeof(media), media_colors));
    for (uint8_t i = 0; i < 9; i++) {
        assert(media_colors[i / 3][i % 3] == i + 1);
    }
    assert(!decode_media_keys(media, 9, media_colors));
    assert(!decode_media_keys(palette, sizeof(palette), media_colors));

    // Golden vector, shared with the host daemon's tests.
    const uint8_t width[32] = {7, 255, 102, 0, 7, 9, 200};
    uint8_t       levels[WIDTH_KEY_COUNT];
    assert(decode_width(width, sizeof(width), levels));
    assert(levels[0] == 255 && levels[1] == 102 && levels[2] == 0 && levels[3] == 7 && levels[4] == 9 && levels[5] == 200);
    assert(!decode_width(width, 6, levels));
    assert(!decode_width(media, sizeof(media), levels));
    // Golden vectors, shared with the host daemon's tests.
    uint8_t       key;
    const uint8_t balance[32] = {8, 0};
    assert(decode_balance(balance, sizeof(balance), &key) && key == 0);
    const uint8_t centre[32] = {8, 127};
    assert(decode_balance(centre, sizeof(centre), &key) && key == 3);
    const uint8_t chat[32] = {8, 255};
    assert(decode_balance(chat, sizeof(chat), &key) && key == BALANCE_KEY_COUNT - 1);
    assert(!decode_balance(chat, 1, &key));
    assert(!decode_balance(media, sizeof(media), &key));

    // Golden vectors, shared with the host daemon's tests.
    bool          on;
    const uint8_t cast_on[32] = {10, 1};
    assert(decode_flag(cast_on, sizeof(cast_on), event_CAST, &on) && on);
    const uint8_t lock_off[32] = {11, 0};
    assert(decode_flag(lock_off, sizeof(lock_off), event_LOCK, &on) && !on);
    assert(!decode_flag(lock_off, sizeof(lock_off), event_CAST, &on));
    assert(!decode_flag(lock_off, 1, event_LOCK, &on));

    // Golden vector, shared with the host daemon's tests.
    uint16_t counts[20] = {1, 258, 65535};
    counts[16]          = 7;
    uint8_t usage[32];
    assert(usage_chunks(20) == 2);
    assert(encode_usage(counts, 20, 0, usage));
    const uint8_t usage_want[32] = {13, 0, 1, 0, 2, 1, 255, 255};
    assert(memcmp(usage, usage_want, sizeof(usage_want)) == 0 && counts[0] == 0 && counts[2] == 0);
    assert(encode_usage(counts, 20, 1, usage));
    assert(usage[1] == 1 && usage[2 + 2 * 1] == 7 && usage[2 + 2 * 5] == 0 && counts[16] == 0);
    assert(!encode_usage(counts, 20, 2, usage));

    // Same chunking for the modifier, dual-role and layer counters, tagged by their own event.
    uint16_t mods[20] = {0, 0, 5};
    uint8_t  tagged[32];
    assert(encode_counts(event_MOD, mods, 20, 0, tagged));
    assert(tagged[0] == event_MOD && tagged[1] == 0 && tagged[2 + 2 * 2] == 5 && mods[2] == 0);
    assert(mod_mask(0x01) == 1 && mod_mask(0x20) == 2 && mod_mask(0x42) == 6 && mod_mask(0x88) == 8 && mod_mask(0) == 0);

    return 0;
}
