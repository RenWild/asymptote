/* Asymptote, a UCI chess engine
   Copyright (C) 2018-2019  Maximilian Lupke

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
use crate::bitboard::*;
use crate::eval::*;
use crate::position::*;
use crate::types::SquareMap;
use rand::{prelude::*, prng::ChaChaRng};

pub fn initialize_magics() {
    let offset = initialize_bishop_attacks(0);
    initialize_rook_attacks(offset);
}

static mut MAGIC_TABLE: [Bitboard; 156_800] = [Bitboard(0); 156_800];
static mut BISHOP_ATTACKS: SquareMap<Magic> = SquareMap::from_array(
    [Magic {
        magic: 0,
        mask: Bitboard(0),
        offset: 0,
    }; 64],
);
static mut ROOK_ATTACKS: SquareMap<Magic> = SquareMap::from_array(
    [Magic {
        magic: 0,
        mask: Bitboard(0),
        offset: 0,
    }; 64],
);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Magic {
    magic: u64,
    mask: Bitboard,
    offset: u32,
}

const SHIFT_MASK: u64 = 0xF8_00_00_00_00_00_00_00;

impl Magic {
    fn index(&self, occupied: Bitboard) -> usize {
        let shift = self.magic.wrapping_shr(56) as u32;
        self.offset as usize
            + ((occupied & self.mask).0.wrapping_mul(self.magic)).wrapping_shr(shift) as usize
    }
}

fn initialize_bishop_attacks(offset: usize) -> usize {
    let border = FILE_A | FILE_H | RANK_1 | RANK_8;

    let mut seed = [0; 32];
    seed[0] = 1;
    for i in 1..32 {
        seed[i] = (((i * i) + seed[i - 1] as usize) % 256) as u8;
    }
    let mut rng = ChaChaRng::from_seed(seed);

    let mut offset = offset;

    for sq in 0..64 {
        let from = Square::from(sq);
        let mask = bishop_from(from, Bitboard::from(0)) & !border;
        let bits = mask.popcount() as u64;
        let shift = 64 - bits;

        let mut occ = Bitboard::from(0);
        let mut size = 0;

        let mut occupancy = Vec::with_capacity(1 << size);
        let mut reference = Vec::with_capacity(1 << size);

        loop {
            occupancy.push(occ);
            reference.push(bishop_from(from, occ));
            size += 1;
            occ = Bitboard::from(occ.0.wrapping_sub(mask.0)) & mask;
            if occ.is_empty() {
                break;
            }
        }

        // search for magics
        let mut magic = Magic {
            magic: sparse_random(&mut rng) & !SHIFT_MASK | shift.wrapping_shl(56),
            mask,
            offset: offset as u32,
        };

        let mut last_used = Vec::with_capacity(size);
        for _ in 0..size {
            last_used.push(0);
        }

        let mut tries = 1;

        'search_magic: loop {
            for i in 0..size {
                let index = magic.index(occupancy[i]);
                let magic_table_entry = unsafe { MAGIC_TABLE[index] };
                if magic_table_entry != reference[i] && last_used[index - offset] == tries {
                    // retry
                    magic.magic = sparse_random(&mut rng) & !SHIFT_MASK | shift.wrapping_shl(56);
                    tries += 1;
                    continue 'search_magic;
                }
                unsafe {
                    MAGIC_TABLE[index] = reference[i];
                }
                last_used[index - offset] = tries;
            }

            break;
        }

        unsafe {
            BISHOP_ATTACKS[from] = magic;
        }
        offset += size;
    }

    offset
}

pub fn get_bishop_attacks_from(from: Square, blockers: Bitboard) -> Bitboard {
    unsafe {
        let magic = &BISHOP_ATTACKS[from];
        *MAGIC_TABLE.get_unchecked(magic.index(blockers))
    }
}

fn bishop_from(from: Square, blockers: Bitboard) -> Bitboard {
    let empty = !blockers;

    let mut propagators_ne = empty;
    let mut propagators_se = empty;
    let mut propagators_sw = empty;
    let mut propagators_nw = empty;
    let mut reachable_ne = Bitboard::from(0);
    let mut reachable_se = Bitboard::from(0);
    let mut reachable_sw = Bitboard::from(0);
    let mut reachable_nw = Bitboard::from(0);
    reachable_ne |= from;
    reachable_se |= from;
    reachable_sw |= from;
    reachable_nw |= from;

    reachable_ne |= reachable_ne.forward(true, 1).right(1) & propagators_ne;
    propagators_ne &= propagators_ne.forward(true, 1).right(1);
    reachable_ne |= reachable_ne.forward(true, 2).right(2) & propagators_ne;
    propagators_ne &= propagators_ne.forward(true, 2).right(2);
    reachable_ne |= reachable_ne.forward(true, 4).right(4) & propagators_ne;

    reachable_se |= reachable_se.backward(true, 1).right(1) & propagators_se;
    propagators_se &= propagators_se.backward(true, 1).right(1);
    reachable_se |= reachable_se.backward(true, 2).right(2) & propagators_se;
    propagators_se &= propagators_se.backward(true, 2).right(2);
    reachable_se |= reachable_se.backward(true, 4).right(4) & propagators_se;

    reachable_sw |= reachable_sw.backward(true, 1).left(1) & propagators_sw;
    propagators_sw &= propagators_sw.backward(true, 1).left(1);
    reachable_sw |= reachable_sw.backward(true, 2).left(2) & propagators_sw;
    propagators_sw &= propagators_sw.backward(true, 2).left(2);
    reachable_sw |= reachable_sw.backward(true, 4).left(4) & propagators_sw;

    reachable_nw |= reachable_nw.forward(true, 1).left(1) & propagators_nw;
    propagators_nw &= propagators_nw.forward(true, 1).left(1);
    reachable_nw |= reachable_nw.forward(true, 2).left(2) & propagators_nw;
    propagators_nw &= propagators_nw.forward(true, 2).left(2);
    reachable_nw |= reachable_nw.forward(true, 4).left(4) & propagators_nw;

    reachable_ne.forward(true, 1).right(1)
        | reachable_se.backward(true, 1).right(1)
        | reachable_sw.backward(true, 1).left(1)
        | reachable_nw.forward(true, 1).left(1)
}

fn initialize_rook_attacks(offset: usize) -> usize {
    let border_files = FILE_A | FILE_H;
    let border_ranks = RANK_1 | RANK_8;

    let mut seed = [0; 32];
    seed[0] = 3;
    for i in 1..32 {
        seed[i] = (((i * i) + seed[i - 1] as usize) % 256) as u8;
    }
    let mut rng = ChaChaRng::from_seed(seed);

    let mut offset = offset;

    for sq in 0..64 {
        let from = Square::from(sq);
        let mask = (FILES[from.file() as usize] & !border_ranks)
            ^ (RANKS[from.rank() as usize] & !border_files);
        let bits = mask.popcount() as u64;
        let shift = 64 - bits;

        let mut occ = Bitboard::from(0);
        let mut size = 0;

        let mut occupancy = Vec::with_capacity(1 << bits);
        let mut reference = Vec::with_capacity(1 << bits);

        loop {
            occupancy.push(occ);
            reference.push(rook_from(from, occ));
            size += 1;
            occ = Bitboard::from(occ.0.wrapping_sub(mask.0)) & mask;
            if occ.is_empty() {
                break;
            }
        }

        // search for magics
        let mut magic = Magic {
            magic: sparse_random(&mut rng) & !SHIFT_MASK | shift.wrapping_shl(56),
            mask,
            offset: offset as u32,
        };

        let mut last_used = Vec::with_capacity(size);
        for _ in 0..size {
            last_used.push(0);
        }

        let mut tries = 1;

        'search_magic: loop {
            for i in 0..size {
                let index = magic.index(occupancy[i]);
                let magic_table_entry = unsafe { MAGIC_TABLE[index] };
                if magic_table_entry != reference[i] && last_used[index - offset] == tries {
                    // retry
                    magic.magic = sparse_random(&mut rng) & !SHIFT_MASK | shift.wrapping_shl(56);
                    tries += 1;
                    continue 'search_magic;
                }
                unsafe {
                    MAGIC_TABLE[index] = reference[i];
                }
                last_used[index - offset] = tries;
            }

            break;
        }

        unsafe {
            ROOK_ATTACKS[from] = magic;
        }
        offset += size;
    }

    offset
}

pub fn get_rook_attacks_from(from: Square, blockers: Bitboard) -> Bitboard {
    unsafe {
        let magic = &ROOK_ATTACKS[from];
        *MAGIC_TABLE.get_unchecked(magic.index(blockers))
    }
}

fn rook_from(from: Square, blockers: Bitboard) -> Bitboard {
    let empty = !blockers;

    let mut propagators_north = empty;
    let mut propagators_south = empty;
    let mut propagators_west = empty;
    let mut propagators_east = empty;
    let mut reachable_north = Bitboard::from(0);
    let mut reachable_south = Bitboard::from(0);
    let mut reachable_west = Bitboard::from(0);
    let mut reachable_east = Bitboard::from(0);
    reachable_north |= from;
    reachable_south |= from;
    reachable_west |= from;
    reachable_east |= from;

    reachable_north |= reachable_north.forward(true, 1) & propagators_north;
    propagators_north &= propagators_north.forward(true, 1);
    reachable_north |= reachable_north.forward(true, 2) & propagators_north;
    propagators_north &= propagators_north.forward(true, 2);
    reachable_north |= reachable_north.forward(true, 4) & propagators_north;

    reachable_south |= reachable_south.backward(true, 1) & propagators_south;
    propagators_south &= propagators_south.backward(true, 1);
    reachable_south |= reachable_south.backward(true, 2) & propagators_south;
    propagators_south &= propagators_south.backward(true, 2);
    reachable_south |= reachable_south.backward(true, 4) & propagators_south;

    reachable_west |= reachable_west.left(1) & propagators_west;
    propagators_west &= propagators_west.left(1);
    reachable_west |= reachable_west.left(2) & propagators_west;
    propagators_west &= propagators_west.left(2);
    reachable_west |= reachable_west.left(4) & propagators_west;

    reachable_east |= reachable_east.right(1) & propagators_east;
    propagators_east &= propagators_east.right(1);
    reachable_east |= reachable_east.right(2) & propagators_east;
    propagators_east &= propagators_east.right(2);
    reachable_east |= reachable_east.right(4) & propagators_east;

    reachable_north.forward(true, 1)
        | reachable_south.backward(true, 1)
        | reachable_west.left(1)
        | reachable_east.right(1)
}

fn sparse_random(rng: &mut ChaChaRng) -> u64 {
    rng.gen::<u64>() & rng.gen::<u64>() & rng.gen::<u64>()
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum Piece {
    Pawn = 0,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Piece {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn value(self) -> Score {
        match self {
            Piece::Pawn => eg(PAWN_SCORE),
            Piece::Knight => eg(KNIGHT_SCORE),
            Piece::Bishop => eg(BISHOP_SCORE),
            Piece::Rook => eg(ROOK_SCORE),
            Piece::Queen => eg(QUEEN_SCORE),
            Piece::King => 10000,
        }
    }

    pub fn see_value(self) -> Score {
        match self {
            Piece::Pawn => 120,
            Piece::Knight => 300,
            Piece::Bishop => 300,
            Piece::Rook => 550,
            Piece::Queen => 1000,
            Piece::King => 10000,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Move {
    pub from: Square,
    pub to: Square,

    pub piece: Piece,
    pub captured: Option<Piece>,
    pub promoted: Option<Piece>,
    pub en_passant: bool,
}

pub struct MoveGenerator<'p> {
    pub position: &'p Position,
}

impl<'p> MoveGenerator<'p> {
    pub fn good_captures(
        &mut self,
        moves: &mut Vec<Move>,
        scores: &mut Vec<i64>,
        bad_moves: &mut Vec<Move>,
        bad_scores: &mut Vec<i64>,
    ) {
        let all_pieces = self.position.all_pieces;
        let them = self.position.them(self.position.white_to_move);
        moves.clear();
        scores.clear();

        if self.position.details.checkers.more_than_one() {
            self.king(them & all_pieces, moves);
        } else if self.position.details.checkers.at_least_one() {
            let checkers = self.position.details.checkers;
            let ep = if self.position.details.en_passant != 255 {
                if self.position.white_to_move {
                    Square::file_rank(self.position.details.en_passant, 5).to_bb()
                } else {
                    Square::file_rank(self.position.details.en_passant, 2).to_bb()
                }
            } else {
                Bitboard::from(0)
            };
            let promotion_rank = if self.position.white_to_move {
                RANK_8
            } else {
                RANK_1
            };

            self.pawn(checkers | promotion_rank | ep, moves);
            self.knight(checkers, moves);
            self.bishop(checkers, moves);
            self.rook(checkers, moves);
            self.queen(checkers, moves);
            self.king(them & all_pieces, moves);
        } else {
            let ep = if self.position.details.en_passant != 255 {
                if self.position.white_to_move {
                    Square::file_rank(self.position.details.en_passant, 5).to_bb()
                } else {
                    Square::file_rank(self.position.details.en_passant, 2).to_bb()
                }
            } else {
                Bitboard::from(0)
            };
            let promotion_rank = if self.position.white_to_move {
                RANK_8
            } else {
                RANK_1
            };

            self.pawn(them & all_pieces | promotion_rank | ep, moves);
            self.knight(them & all_pieces, moves);
            self.bishop(them & all_pieces, moves);
            self.rook(them & all_pieces, moves);
            self.queen(them & all_pieces, moves);
            self.king(them & all_pieces, moves);
        }

        let mut i = 0;
        while i < moves.len() {
            let mov = moves[i];
            if self.position.see(mov, 0) {
                scores.push(mov.mvv_lva_score());
                i += 1;
            } else {
                bad_scores.push(mov.mvv_lva_score());
                bad_moves.push(mov);
                moves.swap_remove(i);
            }
        }
    }

    pub fn quiet_moves(&self, moves: &mut Vec<Move>) {
        moves.clear();
        if self.position.details.checkers.more_than_one() {
            self.king(!self.position.all_pieces, moves);
            return;
        }

        let promotion_rank = if self.position.white_to_move {
            RANK_8
        } else {
            RANK_1
        };

        let ep = if self.position.details.en_passant != 255 {
            if self.position.white_to_move {
                Square::file_rank(self.position.details.en_passant, 5).to_bb()
            } else {
                Square::file_rank(self.position.details.en_passant, 2).to_bb()
            }
        } else {
            Bitboard::from(0)
        };

        self.pawn(!self.position.all_pieces & !promotion_rank & !ep, moves);
        self.knight(!self.position.all_pieces, moves);
        self.bishop(!self.position.all_pieces, moves);
        self.rook(!self.position.all_pieces, moves);
        self.queen(!self.position.all_pieces, moves);
        self.king(!self.position.all_pieces, moves);
    }

    pub fn all_moves(&self) -> Vec<Move> {
        let us = self.position.us(self.position.white_to_move);
        let all = !us;
        let mut moves = Vec::with_capacity(128);
        self.pawn(all, &mut moves);
        self.knight(all, &mut moves);
        self.bishop(all, &mut moves);
        self.rook(all, &mut moves);
        self.queen(all, &mut moves);
        self.king(all, &mut moves);
        moves
    }

    pub fn pawn(&self, targets: Bitboard, moves: &mut Vec<Move>) {
        let us = self.position.us(self.position.white_to_move);
        let them = self.position.them(self.position.white_to_move);
        let promoting = if self.position.white_to_move { 7 } else { 0 };
        let rank3 = if self.position.white_to_move {
            RANK_3
        } else {
            RANK_6
        };

        let wtm = self.position.white_to_move;

        let pawns = self.position.pawns() & us;
        let single_step_targets = pawns.forward(wtm, 1) & !self.position.all_pieces & targets;
        let double_step_targets = (pawns.forward(wtm, 1) & !self.position.all_pieces & rank3)
            .forward(wtm, 1)
            & !self.position.all_pieces
            & targets;
        let captures_left = pawns.forward(wtm, 1).left(1) & them & targets;
        let captures_right = pawns.forward(wtm, 1).right(1) & them & targets;

        for to in single_step_targets.squares() {
            if to.rank() == promoting {
                for promoted in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                    moves.push(Move {
                        from: to.backward(wtm, 1),
                        to,
                        piece: Piece::Pawn,
                        captured: None,
                        promoted: Some(*promoted),
                        en_passant: false,
                    });
                }
            } else {
                moves.push(Move {
                    from: to.backward(wtm, 1),
                    to,
                    piece: Piece::Pawn,
                    captured: None,
                    promoted: None,
                    en_passant: false,
                });
            }
        }

        for to in double_step_targets.squares() {
            moves.push(Move {
                from: to.backward(wtm, 2),
                to,
                piece: Piece::Pawn,
                captured: None,
                promoted: None,
                en_passant: false,
            });
        }

        // en passant
        if self.position.details.en_passant != 255 {
            let en_passant_capturers_rank = if self.position.white_to_move {
                RANK_5
            } else {
                RANK_4
            };
            let ep_square = if self.position.white_to_move {
                Square::file_rank(self.position.details.en_passant, 5)
            } else {
                Square::file_rank(self.position.details.en_passant, 2)
            };
            let capturers = us
                & self.position.pawns()
                & EN_PASSANT_FILES[self.position.details.en_passant as usize]
                & en_passant_capturers_rank;

            if targets & ep_square {
                for from in capturers.squares() {
                    moves.push(Move {
                        from,
                        to: Square::file_rank(
                            self.position.details.en_passant,
                            from.forward(wtm, 1).rank(),
                        ),
                        piece: Piece::Pawn,
                        captured: Some(Piece::Pawn),
                        promoted: None,
                        en_passant: true,
                    });
                }
            }
        }

        // ordinary pawn captures (including promoting captures)
        // captures to the left (file b to file a, ...)
        for to in captures_left.squares() {
            let captured = self.position.find_piece(to);

            if to.rank() == promoting {
                for promoted in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                    moves.push(Move {
                        from: to.backward(self.position.white_to_move, 1).right(1),
                        to,
                        piece: Piece::Pawn,
                        captured,
                        promoted: Some(*promoted),
                        en_passant: false,
                    });
                }
            } else {
                moves.push(Move {
                    from: to.backward(self.position.white_to_move, 1).right(1),
                    to,
                    piece: Piece::Pawn,
                    captured,
                    promoted: None,
                    en_passant: false,
                });
            }
        }

        // captures to the right (file a to file b, ...)
        for to in captures_right.squares() {
            let captured = self.position.find_piece(to);

            if to.rank() == promoting {
                for promoted in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                    moves.push(Move {
                        from: to.backward(wtm, 1).left(1),
                        to,
                        piece: Piece::Pawn,
                        captured,
                        promoted: Some(*promoted),
                        en_passant: false,
                    });
                }
            } else {
                moves.push(Move {
                    from: to.backward(wtm, 1).left(1),
                    to,
                    piece: Piece::Pawn,
                    captured,
                    promoted: None,
                    en_passant: false,
                });
            }
        }
    }

    pub fn knight(&self, targets: Bitboard, moves: &mut Vec<Move>) {
        let us = self.position.us(self.position.white_to_move);
        for from in (self.position.knights() & us).squares() {
            for to in (targets & self.knight_from(from)).squares() {
                moves.push(Move {
                    from,
                    to,
                    piece: Piece::Knight,
                    captured: self.position.find_piece(to),
                    promoted: None,
                    en_passant: false,
                });
            }
        }
    }

    pub fn knight_from(&self, from: Square) -> Bitboard {
        KNIGHT_ATTACKS[from]
    }

    pub fn bishop(&self, targets: Bitboard, moves: &mut Vec<Move>) {
        let us = self.position.us(self.position.white_to_move);
        for from in (self.position.bishops() & us).squares() {
            for to in (targets & get_bishop_attacks_from(from, self.position.all_pieces)).squares()
            {
                moves.push(Move {
                    from,
                    to,
                    piece: Piece::Bishop,
                    captured: self.position.find_piece(to),
                    promoted: None,
                    en_passant: false,
                });
            }
        }
    }

    pub fn rook(&self, targets: Bitboard, moves: &mut Vec<Move>) {
        let us = self.position.us(self.position.white_to_move);
        for from in (self.position.rooks() & us).squares() {
            for to in (targets & get_rook_attacks_from(from, self.position.all_pieces)).squares() {
                moves.push(Move {
                    from,
                    to,
                    piece: Piece::Rook,
                    captured: self.position.find_piece(to),
                    promoted: None,
                    en_passant: false,
                });
            }
        }
    }

    pub fn queen(&self, targets: Bitboard, moves: &mut Vec<Move>) {
        let us = self.position.us(self.position.white_to_move);
        for from in (self.position.queens() & us).squares() {
            for to in (targets
                & (get_bishop_attacks_from(from, self.position.all_pieces)
                    | get_rook_attacks_from(from, self.position.all_pieces)))
            .squares()
            {
                moves.push(Move {
                    from,
                    to,
                    piece: Piece::Queen,
                    captured: self.position.find_piece(to),
                    promoted: None,
                    en_passant: false,
                });
            }
        }
    }

    pub fn king(&self, targets: Bitboard, moves: &mut Vec<Move>) {
        let us = self.position.us(self.position.white_to_move);;
        let castle_kside;
        let castle_qside;
        if self.position.white_to_move {
            castle_kside = (self.position.details.castling & CASTLE_WHITE_KSIDE) > 0
                && (self.position.all_pieces & Bitboard::from(0x00_00_00_00_00_00_00_60))
                    .is_empty()
                && (self.position.rooks() & us & SQUARE_H1);
            castle_qside = (self.position.details.castling & CASTLE_WHITE_QSIDE) > 0
                && (self.position.all_pieces & Bitboard::from(0x00_00_00_00_00_00_00_0E))
                    .is_empty()
                && (self.position.rooks() & us & SQUARE_A1);
        } else {
            castle_kside = (self.position.details.castling & CASTLE_BLACK_KSIDE) > 0
                && (self.position.all_pieces & Bitboard::from(0x60_00_00_00_00_00_00_00))
                    .is_empty()
                && (self.position.rooks() & us & SQUARE_H8);
            castle_qside = (self.position.details.castling & CASTLE_BLACK_QSIDE) > 0
                && (self.position.all_pieces & Bitboard::from(0x0E_00_00_00_00_00_00_00))
                    .is_empty()
                && (self.position.rooks() & us & SQUARE_A8);
        }

        let from = self.position.king_sq(self.position.white_to_move);
        for to in (targets & self.king_from(from)).squares() {
            moves.push(Move {
                from,
                to,
                piece: Piece::King,
                captured: self.position.find_piece(to),
                promoted: None,
                en_passant: false,
            });
        }

        // TODO: check king position?
        if castle_kside && targets & from.right(2) {
            moves.push(Move {
                from,
                to: from.right(2),
                piece: Piece::King,
                captured: None,
                promoted: None,
                en_passant: false,
            });
        }

        if castle_qside && targets & from.left(2) {
            moves.push(Move {
                from,
                to: from.left(2),
                piece: Piece::King,
                captured: None,
                promoted: None,
                en_passant: false,
            });
        }
    }

    pub fn king_from(&self, from: Square) -> Bitboard {
        KING_ATTACKS[from]
    }
}

impl<'p> From<&'p Position> for MoveGenerator<'p> {
    fn from(pos: &'p Position) -> Self {
        MoveGenerator { position: pos }
    }
}

impl Move {
    pub fn is_quiet(self) -> bool {
        self.captured.is_none() && self.promoted.is_none()
    }

    pub fn is_kingside_castle(self) -> bool {
        self.to == self.from.right(2)
    }

    pub fn is_queenside_castle(self) -> bool {
        self.to == self.from.left(2)
    }

    pub fn mvv_lva_score(self) -> i64 {
        let mut score = i64::from(self.captured.map_or(0, Piece::value)) * 128;
        if self.promoted == Some(Piece::Queen) {
            score += i64::from(Piece::Queen.value());
        }
        score -= i64::from(self.piece.value());
        score
    }

    pub fn from_algebraic(pos: &Position, alg: &str) -> Move {
        let mut from_rank = 0;
        let mut from_file = 0;
        let mut to_rank = 0;
        let mut to_file = 0;

        let letters: Vec<_> = "abcdefgh".chars().collect();
        let numbers: Vec<_> = "12345678".chars().collect();

        for (i, &letter) in letters.iter().enumerate() {
            if letter == alg.chars().nth(0).unwrap() {
                from_file = i;
            }

            if letter == alg.chars().nth(2).unwrap() {
                to_file = i;
            }
        }

        for (i, &number) in numbers.iter().enumerate() {
            if number == alg.chars().nth(1).unwrap() {
                from_rank = i;
            }

            if number == alg.chars().nth(3).unwrap() {
                to_rank = i;
            }
        }

        let from = Square::file_rank(from_file as u8, from_rank as u8);
        let to = Square::file_rank(to_file as u8, to_rank as u8);
        let piece = pos.find_piece(from).unwrap();
        let captured;

        let en_passant;
        if piece == Piece::Pawn && !(pos.all_pieces & to) && from.file() != to.file() {
            en_passant = true;
            captured = Some(Piece::Pawn);
        } else {
            en_passant = false;
            captured = pos.find_piece(to);
        }

        let mut promoted = None;
        for &(sym, piece) in &[
            ('q', Piece::Queen),
            ('n', Piece::Knight),
            ('r', Piece::Rook),
            ('b', Piece::Bishop),
        ] {
            if alg.chars().nth(4) == Some(sym) {
                promoted = Some(piece);
                break;
            }
        }

        Move {
            from,
            to,
            piece,
            captured,
            promoted,
            en_passant,
        }
    }

    pub fn to_algebraic(self) -> String {
        let mut alg = String::with_capacity(5);
        let letters: Vec<_> = "abcdefgh".chars().collect();
        let numbers: Vec<_> = "12345678".chars().collect();
        alg.push(letters[self.from.file() as usize]);
        alg.push(numbers[self.from.rank() as usize]);
        alg.push(letters[self.to.file() as usize]);
        alg.push(numbers[self.to.rank() as usize]);
        match self.promoted {
            Some(Piece::Queen) => alg.push('q'),
            Some(Piece::Knight) => alg.push('n'),
            Some(Piece::Rook) => alg.push('r'),
            Some(Piece::Bishop) => alg.push('b'),
            Some(x) => panic!("Invalid promotion piece: {:?}", x),
            None => {}
        }
        alg
    }
}
