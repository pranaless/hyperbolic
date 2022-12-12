use std::{f64::consts::TAU, path::Path};

use cgmath::{Matrix3, One, Rad, Vector2};

use crate::{hyperpoint, translation, Color, Vertex};

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
    color: Color,
    branch: Vec<(u16, u16)>,
}
impl Fragment {
    pub fn parse(s: &str) -> Option<Self> {
        let (color, s) = s.split_once(';')?;
        let color = color.trim().parse::<Color>().ok()?;
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
        Some(Fragment { color, branch })
    }
}

pub struct TilingGenerator {
    data: Vec<Fragment>,
}
impl TilingGenerator {
    pub fn new(s: &str) -> Self {
        let data = s.lines().filter_map(Fragment::parse).collect();
        TilingGenerator { data }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        std::fs::read_to_string(path).map(|s| Self::new(&s))
    }

    pub fn generate(&self, depth: usize) -> (Vec<Vertex>, Vec<u32>) {
        const CENTRAL_ANGLE: f64 = TAU / 4.0;
        const INNER_ANGLE: f64 = TAU / 5.0;
        let len = 2.0 * (1.0 + INNER_ANGLE.cos()) / (1.0 - CENTRAL_ANGLE.cos()) - 1.0;
        let len = (len * len - 1.0).sqrt(); // conv cosh -> sinh

        let mut vertex = Vec::new();
        let mut index = Vec::new();
        let push = |id, origin: Matrix3<f64>| {
            let color = self.data[id as usize].color.into();
            let origin = origin.cast().unwrap();
            let idx = vertex.len() as u32;
            #[rustfmt::skip]
            let pts = [
                origin * hyperpoint(-0.5, -0.5),
                origin * hyperpoint(-0.5,  0.5),
                origin * hyperpoint( 0.5, -0.5),
                origin * hyperpoint( 0.5,  0.5),
            ];
            let pts = [
                Vertex {
                    pos: pts[0].into(),
                    color,
                },
                Vertex {
                    pos: pts[1].into(),
                    color,
                },
                Vertex {
                    pos: pts[2].into(),
                    color,
                },
                Vertex {
                    pos: pts[3].into(),
                    color,
                },
            ];
            vertex.extend_from_slice(&pts);
            index.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 2, idx + 1, idx + 3]);
        };
        let mut state = State {
            rotation_matrix: Matrix3::from_angle_z(Rad(CENTRAL_ANGLE)),
            forward_transform: translation(Vector2::new(len, 0.0)),
            data: &self.data,
            push,
        };
        layer(&mut state, Matrix3::one(), 0, 0, depth);
        (vertex, index)
    }
}
