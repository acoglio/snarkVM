// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use super::*;

impl<E: Environment> One for Field<E> {
    type Boolean = Boolean<E>;
    type Output = Result<Self::Boolean>;

    fn one() -> Self {
        Field::new(Mode::Constant, E::Field::one())
    }

    fn is_one(&self) -> Self::Output {
        unimplemented!()

        // Ok(self.eq(&Self::one())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CircuitBuilder;

    #[test]
    fn test_one() {
        let one = <CircuitBuilder as Environment>::Field::one();

        let candidate = Field::<CircuitBuilder>::one();
        assert_eq!(one, candidate.to_value());
    }

    // #[test]
    // fn test_is_one() -> anyhow::Result<()> {
    //     let candidate = CandidateField::one();
    //
    //     // Should equal 1
    //     let candidate_boolean = candidate.is_one()?;
    //     assert_eq!(true, candidate_boolean.to_value()?);
    //
    //     // Should not equal 0
    //     let candidate_boolean = candidate.is_zero()?;
    //     assert_eq!(false, candidate_boolean.to_value()?);
    //
    //     Ok(())
    // }
}