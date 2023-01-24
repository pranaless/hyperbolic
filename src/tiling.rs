use std::f64::consts::TAU;

use cgmath::{BaseFloat, InnerSpace, Matrix2, Matrix3, One, Rad, Vector2, Vector3, VectorSpace};
use wasm_bindgen::prelude::*;

use crate::{translation, Color, Vertex};

const TURN_AROUND: Matrix3<f64> = Matrix3::new(-1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0);

struct State<'a, F> {
    sides: usize,
    rotation_matrix: Matrix3<f64>,
    forward_transform: Matrix3<f64>,
    data: &'a [Fragment],
    push: F,
}
impl<'a, F> State<'a, F> {
    pub fn iter(&self) -> impl Iterator<Item = (usize, Matrix3<f64>)> {
        let rt = self.rotation_matrix;
        (0..self.sides).scan(self.forward_transform, move |tr, i| {
            let tr1 = *tr;
            *tr = rt * tr1;
            Some((i, tr1))
        })
    }
}

fn layer<F: FnMut(u16, Matrix3<f64>)>(
    state: &mut State<F>,
    tr: Matrix3<f64>,
    id: u16,
    layers: usize,
) {
    (state.push)(id, tr);
    if layers != 0 {
        state
            .iter()
            .filter(|(i, _)| id == 0 || *i != 0)
            .map(|(i, tr)| (state.data[id as usize].branch[i], tr))
            .filter(|(id, _)| *id != 0)
            .for_each(|(id, tr1)| layer(state, tr * tr1, id - 1, layers - 1));
    }
}

struct Fragment {
    branch: Vec<u16>,
}
impl Fragment {
    pub fn parse(s: &str) -> Option<Self> {
        let branch = s
            .split(',')
            .map(|a| a.trim().parse::<u16>().ok().map_or(0, |v| v + 1))
            .collect();
        Some(Fragment { branch })
    }
}

fn kleinpoint<S: BaseFloat>(v: Vector2<S>) -> Vector3<S> {
    let w = S::one() / (S::one() - v.magnitude2()).sqrt();
    v.extend(S::one()) * w
}

struct Mesh<S> {
    vertex: Vec<S>,
    index: Vec<u32>,
}

/// Generate any-sided polygon in the hyperbolic plane.
fn generate_polygon(sides: usize, side: f64, subdiv: usize) -> Mesh<Vector3<f64>> {
    let central_angle = TAU / sides as f64;

    let points = sides * subdiv;
    debug_assert!(points % 2 == 0);
    let mut vertex = Vec::with_capacity(points + 1);
    let mut index = Vec::with_capacity(3 * points);

    let rotation_matrix = Matrix2::from_angle(Rad(central_angle));

    let (s, c) = (0.5 * central_angle).sin_cos();

    vertex.push(Vector3::new(0.0, 0.0, 1.0));
    let mut from;
    let mut to = Vector2::new(-side * c, -side * s);
    for _ in 0..sides {
        from = to;
        to = rotation_matrix * from;
        for i in 0..subdiv {
            let p = i as f64 / subdiv as f64;
            vertex.push(kleinpoint(from.lerp(to, p)));
        }
    }
    for i in 0..points as u32 {
        let j = (i + 1) % points as u32;
        index.extend_from_slice(&[0, 1 + i, 1 + j]);
    }

    Mesh { vertex, index }
}

#[wasm_bindgen]
pub struct TilingGenerator {
    len: f64,
    sides: usize,
    tile: Mesh<Vector3<f64>>,
    data: Vec<Fragment>,
}
#[wasm_bindgen]
impl TilingGenerator {
    #[wasm_bindgen(constructor)]
    pub fn new(p: usize, q: usize, s: &str) -> Self {
        let half_central = TAU / (2.0 * p as f64);
        let half_inner = TAU / (2.0 * q as f64);
        let v = half_inner.cos() / half_central.sin();
        debug_assert!(v >= 1.0);
        let w = (v * v - 1.0).sqrt();
        let side = w / v / half_central.cos();
        let len = 2.0 * v * w;

        let tile = generate_polygon(p, side, 16);

        let data = s.lines().filter_map(Fragment::parse).collect();
        TilingGenerator {
            len,
            sides: p,
            tile,
            data,
        }
    }
}
impl TilingGenerator {
    pub fn generate(&self, colors: &[Color], depth: usize) -> (Vec<Vertex>, Vec<u32>) {
        let central_angle = TAU / self.sides as f64;
        let mut vertex = Vec::new();
        let mut index = Vec::new();
        let push = |id, origin: Matrix3<f64>| {
            let color = colors[id as usize].into();
            let idx = vertex.len() as u32;

            let v = self
                .tile
                .vertex
                .iter()
                .map(|&v| Vertex {
                    pos: (origin * v).cast::<f32>().unwrap().into(),
                    color,
                })
                .collect::<Vec<_>>();
            let i = self.tile.index.iter().map(|&i| idx + i).collect::<Vec<_>>();

            vertex.extend_from_slice(&v);
            index.extend_from_slice(&i);
        };
        let mut state = State {
            sides: self.sides,
            rotation_matrix: Matrix3::from_angle_z(Rad(central_angle)),
            forward_transform: translation(Vector2::new(-self.len, 0.0)) * TURN_AROUND,
            data: &self.data,
            push,
        };
        layer(&mut state, Matrix3::one(), 0, depth);
        (vertex, index)
    }
}
