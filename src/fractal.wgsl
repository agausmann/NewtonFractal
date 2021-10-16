// Compute shader that performs Newton's Fractal for fragments (pixels) of a
// texture.
//
// Complex numbers are used widely in the implementation of Newton's Fractal.
// In this program, the convention is to represent complex numbers as 2D
// vectors, i.e. `vec2<f32>`. A given vector `a` represents the point
// `a.x + i * a.y` in the complex plane.
// 
// Note of caution: The standard multiplication operator `a * b` does not
// implement complex multiplication; instead, the `complex_mul` function must
// be used.

var<private> position_array: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(1.0, -1.0),

    vec2<f32>(1.0, -1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(1.0, 1.0),
);

let MAX_ROOTS: u32 = 10u;
let MAX_COEFFICIENTS: u32 = 11u; // 1u + MAX_ROOTS

struct Root {
    // Color corresponding to this root point. Used to indicate which pixels
    // converge to this root.
    color: vec4<f32>;

    // Position of this root on the complex plane.
    position: vec2<f32>;

    padding: vec2<f32>;
};
[[block]] struct Params {
    // How many Newton-Raphson iterations to perform.
    num_iterations: u32;

    camera_position: vec2<f32>;

    camera_zoom: f32;

    num_roots: u32;

    // The roots of the polynomial, stored contiguously in the lower 0..num_roots elements.
    roots: [[stride(32)]] array<Root, MAX_ROOTS>;

    // Coefficients of the polynomial when written in ascending-power form.
    // The element at array index `i` specifies the coefficient of the term
    // containing the `i` power. There should be `num_roots + 1` coefficients.
    coefficients: [[stride(8)]] array<vec2<f32>, MAX_COEFFICIENTS>;
};
[[group(0), binding(0)]] var<uniform> params: Params;

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] grid_position: vec2<f32>;
};

[[stage(vertex)]]
fn main([[builtin(vertex_index)]] vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position_array[vertex_index], 0.0, 1.0);
    out.grid_position = position_array[vertex_index] / params.camera_zoom + params.camera_position; 
    return out;
}

// Performs complex multiplication of `a` and `b`.
fn complex_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        a.x * b.x - a.y * b.y,
        a.x * b.y + a.y * b.x,
    );
}

// Conjugate of a given complex number `a`.
fn conj(a: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x, -a.y);
}

// Multiplicative inverse of a given complex number `a`.
fn inverse(a: vec2<f32>) -> vec2<f32> {
    return conj(a) / (a.x * a.x + a.y * a.y);
}

// Compute the vaolue of the polynomial specified by `params` at the point `z`.
//
// Uses the factored form of the polynomial: `(z - r1) * (z - r2) * ...`
fn poly(z: vec2<f32>) -> vec2<f32> {
    var product = vec2<f32>(1.0, 0.0);
    for (var i: u32 = 0u; i < params.num_roots; i = i + 1u) {
        product = complex_mul(product, z - params.roots[i].position);
    }
    return product;
}

// Compute the gradient (derivative) of the polynomial specified by `params`
// at the point `z`.
//
// Uses the ascending-powers form of the polynomial, `a0 + a1*z + a2*z^2 + ...`,
// with the derivative being the power rule applied to each term: `a1 + 2*a2*z + ...`
fn grad(z: vec2<f32>) -> vec2<f32> {
    var sum = vec2<f32>(0.0, 0.0);
    var z_power = vec2<f32>(1.0, 0.0);
    for (var i: u32 = 0u; i < params.num_roots; i = i + 1u) {
        sum = sum + f32(i + 1u) * complex_mul(params.coefficients[i + 1u], z_power);
        z_power = complex_mul(z_power, z);
    }
    return sum;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var position: vec2<f32> = in.grid_position;
    for (var i: u32 = 0u; i < params.num_iterations; i = i + 1u) {
        position = position - complex_mul(poly(position), inverse(grad(position)));
    }

    var color: vec4<f32> = params.roots[0].color;
    var min_distance: f32 = distance(position, params.roots[0].position);
    for (var i: u32 = 1u; i < params.num_roots; i = i + 1u) {
        let candidate_distance = distance(position, params.roots[i].position);
        if (candidate_distance < min_distance) {
            min_distance = candidate_distance;
            color = params.roots[i].color;
        }
    }

    return color;
}