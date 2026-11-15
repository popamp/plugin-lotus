struct Frame { time: f32, dt: f32, width: f32, height: f32 }
// Written by pulse.rs: a level per ring, 0 to about 1, and the last kick.
struct Data { level: array<f32, 9>, kick_age: f32, kick: f32, spare: f32 }

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(2) var<storage, read> data: Data;

const RINGS = 9;
const TAU = 6.2831853;
// Ring 0 is the innermost; each ring starts inside the one before it.
const CORE = 0.065;
const FIRST_LENGTH = 0.085;
const LENGTH_GROWTH = 1.2;
const OVERLAP = 0.55;
const FIRST_PETALS = 12.0;
// How far the music pushes a petal out, as a part of its length.
const SWELL = 0.22;
const KICK_SWELL = 0.18;
const KICK_FALL_S = 0.22;
// The kick's wave reaches each ring this much after the one inside it.
const KICK_TRAVEL_S = 0.035;
const TURN_PER_SECOND = 0.012;
const BACKGROUND = vec3f(0.018, 0.018, 0.026);

const PALETTE = array<vec3f, 5>(
    vec3f(0.10, 0.34, 0.38), // teal
    vec3f(0.30, 0.18, 0.44), // violet
    vec3f(0.44, 0.21, 0.14), // rust
    vec3f(0.40, 0.14, 0.31), // plum
    vec3f(0.14, 0.21, 0.44), // blue
);

// One triangle that covers the window.
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    return vec4f(f32(i & 1) * 4.0 - 1.0, f32(i >> 1) * 4.0 - 1.0, 0.0, 1.0);
}

fn hash(n: f32) -> f32 {
    return fract(sin(n * 127.1) * 43758.547);
}

// The kick's push on a ring: none before its wave arrives, then dying away.
fn kick_on(ring: i32) -> f32 {
    let age = data.kick_age - f32(ring) * KICK_TRAVEL_S;
    if age < 0.0 {
        return 0.0;
    }
    return data.kick * smoothstep(0.0, 0.03, age) * exp(-age / KICK_FALL_S);
}

// A petal seen from inside: `along` from its base (0) to its tip (1),
// `across` from its vein (0) to its edge (1, negative on the other side).
struct Petal { along: f32, across: f32, length: f32, id: f32, distance_to_dot: f32 }

// An almond: round at the base, pointed at the tip, widest under a third of the way up.
fn half_width(along: f32) -> f32 {
    return pow(along, 0.55) * pow(1.0 - along, 1.3) * 3.08;
}

// The petal of this ring under the point, if any. Neighbours overlap, so
// the three nearest are tried and the one the point is deepest inside wins.
fn petal_at(p: vec2f, ring: i32, base: f32, length: f32, petals: f32) -> Petal {
    let step = TAU / petals;
    let turn = frame.time * TURN_PER_SECOND * select(1.0, -1.0, ring % 2 == 1) + f32(ring) * 0.5 * step;
    let nearest = round((atan2(p.y, p.x) - turn) / step);
    let width = (base + 0.4 * length) * sin(step * 0.5) * 1.3;
    var best = Petal(-1.0, 2.0, length, 0.0, 1e9);
    for (var j = -1.0; j <= 1.0; j += 1.0) {
        let k = nearest + j;
        let angle = k * step + turn;
        let axis = vec2f(cos(angle), sin(angle));
        let u = dot(p, axis) - base;
        let v = dot(p, vec2f(-axis.y, axis.x));
        let along = u / length;
        if along <= 0.0 || along >= 1.0 {
            continue;
        }
        let across = v / (width * half_width(along));
        if abs(across) < abs(best.across) {
            let id = f32(ring) * 31.0 + ((k % petals) + petals) % petals;
            best = Petal(along, across, length, id, distance(vec2f(u, v), vec2f(0.62 * length, 0.0)));
        }
    }
    return best;
}

fn shade(petal: Petal, ring: i32, swell: f32, pixel: f32) -> vec3f {
    let colour = PALETTE[u32(hash(petal.id) * 4.999)] * (0.85 + 0.3 * hash(petal.id + 0.5));
    let side = abs(petal.across);
    // Dark where the ring inside shades it, lighter towards the tip.
    var c = colour * (0.15 + 0.85 * smoothstep(0.0, 0.8, petal.along)) * (1.0 + 0.5 * swell);
    // Fine veins fanning out from the middle one.
    c *= 0.88 + 0.12 * cos(TAU * (side * 7.0 - petal.along * 3.0));
    c += colour * 0.9 * smoothstep(0.035, 0.0, side) * smoothstep(0.05, 0.3, petal.along) * (1.0 - petal.along);
    // A pale rim, then a dark line where it meets the petal behind.
    c += colour * 0.8 * smoothstep(0.7, 0.93, side);
    let edge = pixel / max(petal.length * 0.15, 1e-4);
    c *= smoothstep(1.0, 1.0 - edge * 2.0, side) * 0.8 + 0.2;
    // A small light near the tip that twinkles, brighter with the music.
    let twinkle = 0.55 + 0.45 * sin(frame.time * (1.3 + hash(petal.id + 0.25)) + hash(petal.id) * TAU);
    let size = 0.004 + 0.012 * petal.length;
    c += vec3f(0.95, 0.92, 1.0) * exp(-pow(petal.distance_to_dot / size, 2.0)) * twinkle * (0.6 + 0.6 * swell);
    return c;
}

// The golden heart: rows of seeds that bulge with the highest band.
fn heart(p: vec2f, r: f32) -> vec3f {
    let seeds = sin(r * 260.0) * sin(atan2(p.y, p.x) * 18.0 + r * 120.0);
    let gold = vec3f(0.78, 0.62, 0.22) * (1.0 + 0.3 * data.level[0]);
    return gold * (0.55 + 0.45 * smoothstep(-0.2, 0.6, seeds)) * (1.0 - 0.5 * r / CORE);
}

@fragment fn fs(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let scale = 0.5 * min(frame.width, frame.height);
    let p = (position.xy - 0.5 * vec2f(frame.width, frame.height)) / scale;
    let pixel = 1.0 / scale;
    let r = length(p);
    if r < CORE {
        return vec4f(heart(p, r), 1.0);
    }
    var base = CORE * 0.8;
    var rest = FIRST_LENGTH;
    for (var ring = 0; ring < RINGS; ring++) {
        let swell = SWELL * clamp(data.level[ring], 0.0, 1.2) + KICK_SWELL * kick_on(ring);
        let length = rest * (1.0 + swell);
        let petal = petal_at(p, ring, base, length, FIRST_PETALS + f32(ring));
        if abs(petal.across) < 1.0 {
            return vec4f(shade(petal, ring, swell / SWELL, pixel), 1.0);
        }
        base += rest * OVERLAP;
        rest *= LENGTH_GROWTH;
    }
    return vec4f(BACKGROUND * (1.0 - 0.4 * r), 1.0);
}
