//! Safe arithmetic references for Qdrant1.19.2 preprocessing; default IEEE environment only.
//! Derived from Qdrant's Apache-2.0 spaces algorithms; see QDRANT-NOTICE.md.
use rom_projection_core::{Error, Result};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Path {
    Scalar,
    Sse,
    Avx,
    Neon,
}
/// Float64 avoids overflow/underflow for every squared finite float32 in the bounded input.
pub(crate) fn normalize(vector: &[f32]) -> Result<Vec<f32>> {
    if !(1..=4096).contains(&vector.len()) || vector.iter().any(|x| !x.is_finite()) {
        return Err(Error::Invalid);
    }
    let length = vector
        .iter()
        .map(|x| {
            let x = f64::from(*x);
            x * x
        })
        .sum::<f64>()
        .sqrt();
    if length == 0. || !length.is_finite() {
        return Err(Error::Invalid);
    }
    Ok(vector
        .iter()
        .map(|x| (f64::from(*x) / length) as f32)
        .collect())
}
pub(crate) fn native_length(vector: &[f32], path: Path) -> f32 {
    if path == Path::Scalar {
        return vector.iter().map(|x| x * x).sum();
    }
    let width = if path == Path::Avx { 8 } else { 4 };
    let block = 4 * width;
    let aligned = vector.len() / block * block;
    let mut groups = [[0_f32; 8]; 4];
    for chunk in vector[..aligned].chunks_exact(block) {
        for (group, lanes) in groups.iter_mut().enumerate() {
            for (lane, old) in lanes.iter_mut().enumerate().take(width) {
                let x = chunk[group * width + lane];
                *old = if path == Path::Sse {
                    x * x + *old
                } else {
                    x.mul_add(x, *old)
                };
            }
        }
    }
    let mut length = if path == Path::Sse {
        let g = groups.map(|a| (a[0] + a[2]) + (a[1] + a[3]));
        ((g[0] + g[1]) + g[2]) + g[3]
    } else {
        let mut s = [0_f32; 8];
        for (i, sum) in s.iter_mut().enumerate().take(width) {
            *sum = (groups[0][i] + groups[1][i]) + (groups[2][i] + groups[3][i]);
        }
        if path == Path::Avx {
            ((s[0] + s[4]) + (s[1] + s[5])) + ((s[2] + s[6]) + (s[3] + s[7]))
        } else {
            (s[0] + s[1]) + (s[2] + s[3])
        }
    };
    // Tagged SIMD tails call powi(2). Cross-build precision is not universally guaranteed.
    for x in &vector[aligned..] {
        length += x.powi(2);
    }
    length
}
fn native(vector: &[f32], path: Path) -> Vec<f32> {
    let length = native_length(vector, path);
    if length < f32::EPSILON || (length - 1.).abs() <= 1e-6 {
        return vector.to_vec();
    }
    let length = length.sqrt();
    if matches!(path, Path::Scalar | Path::Sse) {
        vector.iter().map(|x| x / length).collect()
    } else {
        let inverse = 1. / length;
        vector.iter().map(|x| x * inverse).collect()
    }
}
/// Each result is an entire eligible path. No per-component tolerance or candidate mixing.
pub(crate) fn candidates(vector: &[f32]) -> Vec<Vec<f32>> {
    let mut paths = vec![Path::Scalar];
    if vector.len() >= 16 {
        paths.extend([Path::Sse, Path::Neon]);
    }
    if vector.len() >= 32 {
        paths.push(Path::Avx);
    }
    paths.into_iter().map(|path| native(vector, path)).collect()
}
