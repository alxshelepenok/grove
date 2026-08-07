#[allow(dead_code)]
const VIEW: f64 = 64.0;
const CENTER: f64 = 32.0;
#[allow(dead_code)]
const MARGIN: f64 = 5.0;
const TRUNK_BASE_Y: f64 = 59.0;
const TRUNK_TOP_Y: f64 = 10.0;
const CROTCH_Y: [f64; 3] = [36.0, 22.5, 12.5];
const BASE_HALF: [f64; 3] = [18.5, 15.5, 12.5];
const SAG_RATIO: f64 = 0.8;
const CTRL_FRAC: f64 = 0.2;
const LEN_MIN: f64 = 0.8;
const LEN_MAX: f64 = 1.2;
const TRUNK_W_MIN: f64 = 5.6;
#[allow(dead_code)]
const TRUNK_W_MAX: f64 = 7.2;
const TIER_W_MIN: f64 = 4.2;
#[allow(dead_code)]
const TIER_W_MAX: f64 = 5.6;
const BRANCH_W_MIN: f64 = 4.0;
const BRANCH_W_MAX: f64 = 6.0;

const DEEP_TONES: [[i64; 3]; 7] = [
    [262, 48, 32],
    [184, 55, 30],
    [191, 42, 28],
    [166, 45, 28],
    [43, 55, 32],
    [342, 50, 36],
    [280, 42, 34],
];

const LIGHT_TONES: [[i64; 3]; 7] = [
    [262, 83, 78],
    [184, 96, 70],
    [184, 80, 66],
    [166, 90, 66],
    [43, 95, 66],
    [342, 100, 76],
    [280, 65, 74],
];

#[allow(dead_code)]
const FAMILY_HUES: [i64; 7] = [262, 184, 191, 166, 43, 342, 280];

fn hash_code(name: &str) -> u32 {
    let mut hash: i32 = 0;
    for unit in name.encode_utf16() {
        hash = (hash << 5).wrapping_sub(hash).wrapping_add(i32::from(unit));
    }
    hash.unsigned_abs()
}

fn get_digit(number: u32, ntn: u32) -> u32 {
    (number / 10u32.pow(ntn)) % 10
}

fn get_unit(number: u32, range: u32, index: u32) -> i64 {
    let value = i64::from(number % range);
    if index != 0 && get_digit(number, index) % 2 == 0 {
        -value
    } else {
        value
    }
}

struct Draws<'a> {
    name: &'a str,
    salt: u32,
}

impl<'a> Draws<'a> {
    fn new(name: &'a str) -> Draws<'a> {
        Draws { name, salt: 0 }
    }

    fn draw(&mut self, range: u32, index: u32) -> i64 {
        let h = hash_code(&format!("{}:{}", self.name, self.salt));
        self.salt += 1;
        get_unit(h, range, index)
    }

    fn next(&mut self, range: u32) -> i64 {
        self.draw(range, 0)
    }

    fn signed(&mut self, range: u32, index: u32) -> i64 {
        self.draw(range, index)
    }
}

fn yiq(r: f64, g: f64, b: f64) -> f64 {
    (r * 299.0 + g * 587.0 + b * 114.0) / 1000.0
}

fn hsl_yiq(tone: [i64; 3]) -> f64 {
    let h = tone[0] as f64;
    let sn = tone[1] as f64 / 100.0;
    let ln = tone[2] as f64 / 100.0;
    let c = (1.0 - (2.0 * ln - 1.0).abs()) * sn;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = ln - c / 2.0;
    let rgb = match ((h / 60.0).floor() as i64).rem_euclid(6) {
        0 => [c, x, 0.0],
        1 => [x, c, 0.0],
        2 => [0.0, c, x],
        3 => [0.0, x, c],
        4 => [x, 0.0, c],
        _ => [c, 0.0, x],
    };
    yiq(
        ((rgb[0] + m) * 255.0).round(),
        ((rgb[1] + m) * 255.0).round(),
        ((rgb[2] + m) * 255.0).round(),
    )
}

fn pick_tone(draw: &mut Draws, tones: &[[i64; 3]; 7], bg_yiq: f64) -> [i64; 3] {
    let mut idx = draw.next(tones.len() as u32) as usize;
    for _ in 0..tones.len() {
        if (hsl_yiq(tones[idx]) - bg_yiq).abs() >= 90.0 {
            return tones[idx];
        }
        idx = (idx + 1 + draw.next((tones.len() - 1) as u32) as usize) % tones.len();
    }
    tones[idx]
}

fn css_hsl(tone: [i64; 3]) -> String {
    format!("hsl({}, {}%, {}%)", tone[0], tone[1], tone[2])
}

fn num(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == r.trunc() {
        return format!("{}", r as i64);
    }
    format!("{:.2}", r)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

struct Bough {
    width: f64,
    tip_x: f64,
    tip_y: f64,
    ctrl_x: f64,
    ctrl_y: f64,
}

struct Tier {
    crotch_y: f64,
    #[allow(dead_code)]
    half: f64,
    left: Bough,
    right: Bough,
}

struct Layout {
    bg: [i64; 3],
    wood: [i64; 3],
    foliage: [i64; 3],
    trunk_w: f64,
    tiers: Vec<Tier>,
}

fn tree_layout(name: &str) -> Layout {
    let hash = hash_code(name);
    let mut draw = Draws::new(name);
    let bg = DEEP_TONES[(hash % DEEP_TONES.len() as u32) as usize];
    let bg_yiq = hsl_yiq(bg);
    let wood = pick_tone(&mut draw, &LIGHT_TONES, bg_yiq);
    let foliage = pick_tone(&mut draw, &LIGHT_TONES, bg_yiq);
    let trunk_w = TRUNK_W_MIN + draw.next(17) as f64 / 10.0;

    let mut tiers = Vec::with_capacity(CROTCH_Y.len());
    for i in 0..CROTCH_Y.len() {
        let factor = LEN_MIN + draw.next(41) as f64 / 100.0 * (LEN_MAX - LEN_MIN);
        let half = BASE_HALF[i] * factor;
        let tier_w = TIER_W_MIN + draw.next(15) as f64 / 10.0;
        let jitter_left = draw.signed(5, 1) as f64 / 10.0;
        let jitter_right = draw.signed(5, 1) as f64 / 10.0;
        let tip_y = CROTCH_Y[i] + half * SAG_RATIO;
        let bough = |side: f64, jitter: f64| Bough {
            width: (tier_w + jitter).clamp(BRANCH_W_MIN, BRANCH_W_MAX),
            tip_x: CENTER + side * half,
            tip_y,
            ctrl_x: CENTER + side * half * CTRL_FRAC,
            ctrl_y: tip_y,
        };
        tiers.push(Tier {
            crotch_y: CROTCH_Y[i],
            half,
            left: bough(-1.0, jitter_left),
            right: bough(1.0, jitter_right),
        });
    }

    Layout {
        bg,
        wood,
        foliage,
        trunk_w,
        tiers,
    }
}

pub fn tree_avatar_svg(name: &str) -> String {
    let layout = tree_layout(name);
    let mut out = String::with_capacity(2048);
    out.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">"#);
    out.push_str(&format!(
        r#"<rect width="64" height="64" rx="0" fill="{}"/>"#,
        css_hsl(layout.bg)
    ));
    out.push_str(r#"<g fill="none" stroke-linecap="round">"#);
    for tier in &layout.tiers {
        for bough in [&tier.left, &tier.right] {
            out.push_str(&format!(
                r#"<path d="M{} {}Q{} {} {} {}" stroke="{}" stroke-width="{}"/>"#,
                num(CENTER),
                num(tier.crotch_y),
                num(bough.ctrl_x),
                num(bough.ctrl_y),
                num(bough.tip_x),
                num(bough.tip_y),
                css_hsl(layout.foliage),
                num(bough.width)
            ));
        }
    }
    out.push_str(&format!(
        r#"<path d="M{} {}L{} {}" stroke="{}" stroke-width="{}"/>"#,
        num(CENTER),
        num(TRUNK_BASE_Y),
        num(CENTER),
        num(TRUNK_TOP_Y),
        css_hsl(layout.wood),
        num(layout.trunk_w)
    ));
    out.push_str("</g></svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: [&str; 12] = [
        "Alpha Pines",
        "Beta Willows",
        "Gamma Falls",
        "Delta Dunes",
        "Echo Harbor",
        "Fern Gully",
        "Golden Meadow",
        "Harbor Lights",
        "Indigo Peak",
        "Juniper Bay",
        "Kelp Forest",
        "Lunar Basin",
    ];

    fn hues_in(svg: &str) -> Vec<i64> {
        svg.split("hsl(")
            .skip(1)
            .map(|rest| {
                rest.split(',')
                    .next()
                    .unwrap()
                    .trim()
                    .parse::<i64>()
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn repeat_renders_are_byte_identical() {
        for name in SAMPLE {
            let a = tree_avatar_svg(name);
            assert_eq!(a, tree_avatar_svg(name), "{name} render diverged");
            assert_eq!(a, tree_avatar_svg(name), "{name} render diverged");
        }
        assert_eq!(tree_avatar_svg(""), tree_avatar_svg(""));
        let unicode = "Baume 日本";
        assert_eq!(tree_avatar_svg(unicode), tree_avatar_svg(unicode));
    }

    #[test]
    fn distinct_names_render_distinct_svgs() {
        let svgs: std::collections::HashSet<String> =
            SAMPLE.into_iter().map(tree_avatar_svg).collect();
        assert_eq!(svgs.len(), SAMPLE.len(), "each sample name keeps its own tree");
    }

    #[test]
    fn palette_hues_stay_within_design_families() {
        for name in SAMPLE {
            let svg = tree_avatar_svg(name);
            let hues = hues_in(&svg);
            assert!(hues.len() >= 3, "{name} tile, wood and foliage tones present");
            for h in hues {
                assert!(FAMILY_HUES.contains(&h), "{name} hue {h} outside the family list");
            }
        }
    }

    #[test]
    fn pine_structure_stacks_trunk_and_three_bough_tiers() {
        assert!(TRUNK_TOP_Y < CROTCH_Y[2], "leader pokes above the top tier");
        assert!(
            CROTCH_Y[2] < CROTCH_Y[1] && CROTCH_Y[1] < CROTCH_Y[0],
            "tiers stack up the trunk"
        );
        for name in SAMPLE {
            let layout = tree_layout(name);
            assert_eq!(layout.tiers.len(), 3, "{name} keeps three bough tiers");
            let svg = tree_avatar_svg(name);
            assert_eq!(svg.matches("<path").count(), 7, "{name} trunk plus six boughs");
            assert_eq!(svg.matches('Q').count(), 6, "{name} boughs are quadratic curves");
            for tier in &layout.tiers {
                assert!(
                    (tier.left.tip_x + tier.right.tip_x - VIEW).abs() < 1e-9,
                    "{name} tier pair mirrors across the trunk"
                );
                assert!(
                    (tier.left.ctrl_x + tier.right.ctrl_x - VIEW).abs() < 1e-9,
                    "{name} tier controls mirror across the trunk"
                );
                assert_eq!(tier.left.tip_y, tier.right.tip_y, "{name} pair tips level");
                assert!(
                    tier.left.tip_x < CENTER && tier.right.tip_x > CENTER,
                    "{name} pair straddles the trunk"
                );
                assert!(
                    tier.left.tip_y > tier.crotch_y,
                    "{name} boughs sweep out like the reference pine"
                );
            }
        }
    }

    #[test]
    fn bough_lengths_vary_per_tier_within_bounds() {
        let mut seen: [std::collections::HashSet<i64>; 3] = Default::default();
        for name in SAMPLE {
            let layout = tree_layout(name);
            for (i, tier) in layout.tiers.iter().enumerate() {
                let half = tier.right.tip_x - CENTER;
                assert!(
                    half >= BASE_HALF[i] * LEN_MIN - 1e-9,
                    "{name} tier {i} half {half} below the seeded floor"
                );
                assert!(
                    half <= BASE_HALF[i] * LEN_MAX + 1e-9,
                    "{name} tier {i} half {half} above the seeded cap"
                );
                seen[i].insert((half * 100.0).round() as i64);
            }
        }
        for (i, set) in seen.iter().enumerate() {
            assert!(
                set.len() >= 8,
                "tier {i} length only takes {} values across the sample",
                set.len()
            );
        }
    }

    #[test]
    fn stroke_thickness_stays_seeded_and_legible() {
        assert!(
            BRANCH_W_MIN * 24.0 / VIEW >= 1.5,
            "thinnest bough clears 1.5px at 24px"
        );
        assert!(TRUNK_W_MIN * 24.0 / VIEW >= 2.0, "trunk clears 2px at 24px");
        let mut trunk_widths = std::collections::HashSet::new();
        let mut bough_widths = std::collections::HashSet::new();
        for name in SAMPLE {
            let layout = tree_layout(name);
            assert!(
                layout.trunk_w >= TRUNK_W_MIN - 1e-9 && layout.trunk_w <= TRUNK_W_MAX + 1e-9,
                "{name} trunk width {}",
                layout.trunk_w
            );
            trunk_widths.insert((layout.trunk_w * 100.0).round() as i64);
            for tier in &layout.tiers {
                for bough in [&tier.left, &tier.right] {
                    assert!(
                        bough.width >= BRANCH_W_MIN - 1e-9,
                        "{name} bough width {} under 4 units",
                        bough.width
                    );
                    assert!(
                        bough.width <= BRANCH_W_MAX + 1e-9,
                        "{name} bough width {} over the cap",
                        bough.width
                    );
                    bough_widths.insert((bough.width * 100.0).round() as i64);
                }
            }
        }
        assert!(
            trunk_widths.len() >= 6,
            "trunk thickness varies across names: {}",
            trunk_widths.len()
        );
        assert!(
            bough_widths.len() >= 12,
            "bough thickness varies across names: {}",
            bough_widths.len()
        );
    }

    #[test]
    fn bough_tiers_never_tangle() {
        let top_deepest = CROTCH_Y[2] + BASE_HALF[2] * LEN_MAX * SAG_RATIO;
        let mid_shallowest = CROTCH_Y[1] + BASE_HALF[1] * LEN_MIN * SAG_RATIO;
        let mid_deepest = CROTCH_Y[1] + BASE_HALF[1] * LEN_MAX * SAG_RATIO;
        let bot_shallowest = CROTCH_Y[0] + BASE_HALF[0] * LEN_MIN * SAG_RATIO;
        assert!(
            top_deepest < mid_shallowest,
            "top tier can never reach the middle band"
        );
        assert!(
            mid_deepest < bot_shallowest,
            "middle tier can never reach the bottom band"
        );
        for name in SAMPLE {
            let layout = tree_layout(name);
            let top = layout.tiers[2].left.tip_y;
            let mid = layout.tiers[1].left.tip_y;
            let bot = layout.tiers[0].left.tip_y;
            assert!(top < mid && mid < bot, "{name} tier tips keep vertical order");
            let bare = TRUNK_BASE_Y - (bot + layout.tiers[0].left.width / 2.0);
            assert!(bare >= 2.0, "{name} bare trunk shows below the boughs");
        }
    }

    #[test]
    fn trunk_anchors_bottom_and_stays_on_the_tile() {
        assert!(
            TRUNK_TOP_Y - TRUNK_W_MAX / 2.0 >= MARGIN - 0.01,
            "leader cap stays on the tile"
        );
        for name in SAMPLE {
            let layout = tree_layout(name);
            let svg = tree_avatar_svg(name);
            assert!(
                svg.contains(&format!(
                    "M{} {}L{} {}",
                    num(CENTER),
                    num(TRUNK_BASE_Y),
                    num(CENTER),
                    num(TRUNK_TOP_Y)
                )),
                "{name} trunk runs the full center line"
            );
            let deepest = layout.tiers[0].left.tip_y + layout.tiers[0].left.width / 2.0;
            assert!(
                deepest < TRUNK_BASE_Y,
                "{name} trunk base sits below every bough"
            );
            for tier in &layout.tiers {
                for bough in [&tier.left, &tier.right] {
                    assert!(
                        bough.tip_x - bough.width / 2.0 >= MARGIN - 0.01,
                        "{name} bough tip exits the tile west or east"
                    );
                    assert!(
                        bough.tip_x + bough.width / 2.0 <= VIEW - MARGIN + 0.01,
                        "{name} bough tip exits the tile west or east"
                    );
                    assert!(
                        bough.tip_y + bough.width / 2.0 <= VIEW - MARGIN + 0.01,
                        "{name} bough tip exits the tile south"
                    );
                }
            }
        }
    }

    #[test]
    fn tones_clear_the_tile_contrast_guard() {
        for name in SAMPLE {
            let layout = tree_layout(name);
            let bg = hsl_yiq(layout.bg);
            assert!(
                (hsl_yiq(layout.wood) - bg).abs() >= 90.0,
                "{name} wood clashes with the tile"
            );
            assert!(
                (hsl_yiq(layout.foliage) - bg).abs() >= 90.0,
                "{name} foliage clashes with the tile"
            );
        }
    }

    #[test]
    fn generator_draws_only_from_the_seed() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/avatar.rs"))
            .unwrap();
        for banned in [
            concat!("Math.", "random"),
            concat!("thread", "_rng"),
            concat!("ra", "nd::"),
            concat!("System", "Time"),
            concat!("Inst", "ant"),
        ] {
            assert!(!src.contains(banned), "avatar.rs stays seeded only: {banned}");
        }
    }

    #[test]
    fn svg_output_is_ascii_only() {
        for name in SAMPLE {
            assert!(tree_avatar_svg(name).is_ascii(), "{name} svg carries non-ascii bytes");
        }
        assert!(tree_avatar_svg("Baume 日本").is_ascii());
    }

    #[test]
    fn svg_interpolates_numbers_and_palette_only() {
        for name in SAMPLE {
            let svg = tree_avatar_svg(name);
            assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">"#));
            assert!(svg.ends_with("</svg>"));
            assert!(!svg.contains(name), "{name} never reaches the markup");
        }
        let hostile = tree_avatar_svg("\"><script>alert(1)</script>");
        assert!(!hostile.contains("<script>"));
        assert!(hostile.is_ascii());
    }

    #[test]
    fn svg_marks_up_tile_trunk_and_bough_tiers() {
        let svg = tree_avatar_svg("Alpha Pines");
        assert!(svg.contains(r#"<rect width="64" height="64" rx="0" fill="hsl("#));
        assert!(svg.contains(r#"stroke-linecap="round"#), "caps stay rounded");
        assert!(svg.contains(r#"fill="none"#), "strokes only");
        assert!(!svg.contains("<circle"), "blob crown is gone");
        assert!(!svg.contains("fill-opacity"), "blob crown is gone");
        assert_eq!(svg.matches("stroke-width=").count(), 7, "trunk and six bough widths");
        assert!(svg.contains('Q'), "boughs curve");
        assert!(svg.contains('L'), "trunk is straight");
    }
}
