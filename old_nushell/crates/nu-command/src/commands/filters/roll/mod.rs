mod column;
mod command;
mod up;

pub use column::SubCommand as RollColumn;
pub use command::Command as Roll;
pub use up::SubCommand as RollUp;

mod support {

    pub enum Direction {
        Left,
        Right,
        Down,
        Up,
    }

    pub fn rotate<T: Clone>(
        mut collection: Vec<T>,
        n: &Option<nu_source::Tagged<u64>>,
        direction: Direction,
    ) -> Option<Vec<T>> {
        if collection.is_empty() {
            return None;
        }

        let values = collection.as_mut_slice();

        let rotations = if let Some(n) = n {
            n.item as usize % values.len()
        } else {
            1
        };

        match direction {
            Direction::Up | Direction::Right => values.rotate_left(rotations),
            Direction::Down | Direction::Left => values.rotate_right(rotations),
        }

        Some(values.to_vec())
    }
}
