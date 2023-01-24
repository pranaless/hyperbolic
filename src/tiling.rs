use std::{f64::consts::TAU, path::Path};

use cgmath::{BaseFloat, Matrix3, One, Rad, Vector2, Vector3};

use crate::{translation, Color, Vertex};

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
            .filter(|(i, _)| id == 0 || *i != 2)
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

fn kleinpoint<S: BaseFloat>(x: S, y: S) -> Vector3<S> {
    let w = S::one() / (S::one() - x * x - y * y).sqrt();
    Vector3::new(x * w, y * w, w)
}

fn lerp<S: BaseFloat>(a: S, b: S, p: S) -> S {
    a * (S::one() - p) + b * p
}

struct Mesh<S> {
    vertex: Vec<S>,
    index: Vec<u32>,
}

pub struct TilingGenerator {
    len: f64,
    tile: Mesh<Vector3<f32>>,
    data: Vec<Fragment>,
}
impl TilingGenerator {
    const CENTRAL_ANGLE: f64 = TAU / 4.0;
    const INNER_ANGLE: f64 = TAU / 5.0;

    fn generate_tile<S: BaseFloat>(side: S, subdiv: usize) -> Mesh<Vector3<S>> {
        let mut vertex = Vec::with_capacity(4 * subdiv);
        let mut index = Vec::with_capacity(6 * subdiv);

        for i in 0..subdiv {
            let p = S::from(i).unwrap() / S::from(subdiv).unwrap();
            vertex.push(kleinpoint(-side, lerp(-side, side, p)));
        }
        for i in 0..subdiv {
            let p = S::from(i).unwrap() / S::from(subdiv).unwrap();
            vertex.push(kleinpoint(lerp(-side, side, p), side));
        }
        for i in 0..subdiv {
            let p = S::from(i).unwrap() / S::from(subdiv).unwrap();
            vertex.push(kleinpoint(side, lerp(side, -side, p)));
        }
        for i in 0..subdiv {
            let p = S::from(i).unwrap() / S::from(subdiv).unwrap();
            vertex.push(kleinpoint(lerp(side, -side, p), -side));
        }
        for i in 0..2 * subdiv as u32 {
            let j = 39 - i;
            index.extend_from_slice(&[i + 1, i, j, i + 1, j, j - 1]);
        }

        Mesh { vertex, index }
    }

    pub fn new(s: &str) -> Self {
        let len = (1.0 + Self::INNER_ANGLE.cos()) / (1.0 - Self::CENTRAL_ANGLE.cos());
        let side = (1.0 - 1.0 / len).sqrt() as f32;
        let len = 2.0 * (len * len - len).sqrt();

        let tile = Self::generate_tile(side, 10);

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
            let origin = origin.cast().unwrap();
            let idx = vertex.len() as u32;

            let v = self
                .tile
                .vertex
                .iter()
                .map(|&v| Vertex {
                    pos: (origin * v).into(),
                    color,
                })
                .collect::<Vec<_>>();
            let i = self.tile.index.iter().map(|&i| idx + i).collect::<Vec<_>>();

            vertex.extend_from_slice(&v);
            index.extend_from_slice(&i);
        };
        let mut state = State {
            rotation_matrix: Matrix3::from_angle_z(Rad(Self::CENTRAL_ANGLE)),
            forward_transform: translation(Vector2::new(self.len, 0.0)),
            data: &self.data,
            push,
        };
        layer(&mut state, Matrix3::one(), 0, 0, depth);
        (vertex, index)
    }
}
