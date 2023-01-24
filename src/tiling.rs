use std::{f64::consts::TAU, path::Path};

use cgmath::{BaseFloat, InnerSpace, Matrix2, Matrix3, One, Rad, Vector2, Vector3, VectorSpace};

use crate::{translation, Color, Vertex};

const TURN_AROUND: Matrix3<f64> = Matrix3::new(-1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0);

struct State<'a, F> {
    rotation_matrix: Matrix3<f64>,
    forward_transform: Matrix3<f64>,
    data: &'a [Fragment],
    push: F,
}
impl<'a, F> State<'a, F> {
    pub fn iter(&self) -> impl Iterator<Item = (usize, Matrix3<f64>)> {
        let rt = self.rotation_matrix;
        (0..4).scan(self.forward_transform, move |tr, i| {
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
    rot: u16,
    layers: usize,
) {
    (state.push)(id, tr);
    if layers != 0 {
        state
            .iter()
            .filter(|(i, _)| id == 0 || *i != 0)
            .map(|(i, tr)| (state.data[id as usize].branch[(i + rot as usize) % 4], tr))
            .filter(|(id, _)| id.0 != 0)
            .for_each(|(id, tr1)| layer(state, tr * tr1, id.0 - 1, id.1, layers - 1));
    }
}

struct Fragment {
    branch: Vec<(u16, u16)>,
}
impl Fragment {
    pub fn parse(s: &str) -> Option<Self> {
        let branch = s
            .split(',')
            .map(|v| v.split_once('+').unwrap_or((v, "0")))
            .map(|(a, b)| {
                (
                    a.trim().parse::<u16>().ok().map_or(0, |v| v + 1),
                    b.trim().parse::<u16>().ok().unwrap_or(0),
                )
            })
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
    let mut vertex = Vec::with_capacity(points);
    let mut index = Vec::with_capacity(3 * points);

    let rotation_matrix = Matrix2::from_angle(Rad(central_angle));

    let (s, c) = (0.5 * central_angle).sin_cos();

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
    for i in 0..points as u32 / 2 {
        let j = points as u32 - i - 1;
        index.extend_from_slice(&[i + 1, i, j, i + 1, j, j - 1]);
    }

    Mesh { vertex, index }
}

pub struct TilingGenerator {
    len: f64,
    tile: Mesh<Vector3<f64>>,
    data: Vec<Fragment>,
}
impl TilingGenerator {
    const HALF_CENTRAL_ANGLE: f64 = TAU / 8.0;
    const HALF_INNER_ANGLE: f64 = TAU / 10.0;

    pub fn new(s: &str) -> Self {
        let v = Self::HALF_INNER_ANGLE.cos() / Self::HALF_CENTRAL_ANGLE.sin();
        debug_assert!(v >= 1.0);
        let w = (v * v - 1.0).sqrt();
        let side = w / v / Self::HALF_CENTRAL_ANGLE.cos();
        let len = 2.0 * v * w;

        let tile = generate_polygon(4, side, 4);

        let data = s.lines().filter_map(Fragment::parse).collect();
        TilingGenerator { len, tile, data }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        std::fs::read_to_string(path).map(|s| Self::new(&s))
    }

    pub fn generate(&self, colors: &[Color], depth: usize) -> (Vec<Vertex>, Vec<u32>) {
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
            rotation_matrix: Matrix3::from_angle_z(Rad(2.0 * Self::HALF_CENTRAL_ANGLE)),
            forward_transform: translation(Vector2::new(self.len, 0.0)) * TURN_AROUND,
            data: &self.data,
            push,
        };
        layer(&mut state, Matrix3::one(), 0, 0, depth);
        (vertex, index)
    }
}
