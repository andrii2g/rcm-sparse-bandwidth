use rand::Rng;
use rand::seq::SliceRandom;

/// A validated symmetric matrix permutation and its inverse.
#[derive(Debug)]
pub struct Permutation {
    new_to_old: Vec<usize>,
    old_to_new: Vec<usize>,
}

impl Permutation {
    pub fn new(new_to_old: Vec<usize>, vertex_count: usize) -> Result<Self, String> {
        if new_to_old.len() != vertex_count {
            return Err(format!(
                "expected {vertex_count} permutation indices, got {}",
                new_to_old.len()
            ));
        }
        let mut old_to_new = vec![vertex_count; vertex_count];
        for (new, &old) in new_to_old.iter().enumerate() {
            let index = old_to_new
                .get_mut(old)
                .ok_or_else(|| format!("permutation index {old} is outside 0..{vertex_count}"))?;
            if *index != vertex_count {
                return Err(format!("duplicate permutation index {old}"));
            }
            *index = new;
        }
        Ok(Self {
            new_to_old,
            old_to_new,
        })
    }

    pub fn new_to_old(&self) -> &[usize] {
        &self.new_to_old
    }

    pub fn old_to_new(&self) -> &[usize] {
        &self.old_to_new
    }
}

pub fn shuffled_indices(vertex_count: usize, rng: &mut impl Rng) -> Vec<usize> {
    let mut indices: Vec<_> = (0..vertex_count).collect();
    indices.shuffle(rng);
    indices
}
