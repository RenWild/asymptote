/* Asymptote, a UCI chess engine
   Copyright (C) 2018  Maximilian Lupke

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use eval::*;
use movegen::*;
use search::*;

pub struct History {
    from_to: [[[i32; 64]; 64]; 2],
}

impl Default for History {
    fn default() -> History {
        History {
            from_to: [[[0; 64]; 64]; 2],
        }
    }
}

impl History {
    pub fn get_score(&self, white: bool, mov: Move) -> i32 {
        self.from_to[white as usize][mov.from.0 as usize][mov.to.0 as usize]
    }

    pub fn increase_score(&mut self, white: bool, mov: Move, depth: Depth) {
        let d = i32::from(depth / INC_PLY);
        let rescale;

        {
            let entry = &mut self.from_to[white as usize][mov.from.0 as usize][mov.to.0 as usize];
            *entry += d*d;
            rescale = *entry > i32::from(Score::max_value());
        }

        if rescale {
            for stm in 0..2 {
                for from in 0..64 {
                    for to in 0..64 {
                        self.from_to[stm][from][to] /= 4;
                    }
                }
            }
        }
    }
}