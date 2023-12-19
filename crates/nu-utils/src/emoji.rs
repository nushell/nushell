pub fn contains_emoji(val: &str) -> bool {
    // Let's do some special handling for emojis
    const ZERO_WIDTH_JOINER: &str = "\u{200d}";
    const VARIATION_SELECTOR_16: &str = "\u{fe0f}";
    const SKIN_TONES: [&str; 5] = [
        "\u{1f3fb}", // Light Skin Tone
        "\u{1f3fc}", // Medium-Light Skin Tone
        "\u{1f3fd}", // Medium Skin Tone
        "\u{1f3fe}", // Medium-Dark Skin Tone
        "\u{1f3ff}", // Dark Skin Tone
    ];

    val.contains(ZERO_WIDTH_JOINER)
        || val.contains(VARIATION_SELECTOR_16)
        || SKIN_TONES.iter().any(|skin_tone| val.contains(skin_tone))
}
