use std::time::Instant;

use clap::Parser;

/// Do an iteration of the search algorithm
///
/// `hand` is the current partial hand, represented as a bitvector.
/// `distances[i]` is the number of pairs of cards in the hand whose XOR is `i`.
/// `next_index` is the next card to (maybe) add.
/// `max_index` is the size of the deck.
/// `cards_to_add` is the number of cards left to add.
/// `quads` is the number of quads in the current partial hand times 3.
/// `best_quads` is the highest number of quads in a hand found so far.
/// `best_table` is the hand with the highest number of quads in a hand found so far.
fn iteration(
    hand: u128,
    distances: [u8; 128],
    next_index: u8,
    max_index: u8,
    cards_to_add: u8,
    quads: u64,
    best_quads: &mut u64,
    best_hand: &mut u128,
) {
    // Nothing useful to do.
    if next_index + cards_to_add > max_index || cards_to_add == 0 {
        return;
    }
    let last_card_in_hand = hand.checked_ilog2().unwrap_or(0);
    // The maximum dimension of the vector space used by a card in the hand,
    // indexed from 0.
    let max_dimension_used = last_card_in_hand.checked_ilog2().unwrap_or(0);
    // Don't allow adding a card that adds a new dimension to the hand unless
    // it's the first possible such card. (For example, if the highest card in
    // the hand is 3, there's no point adding any card above 4 since you could
    // equivalently add card 4 instead.)
    let max_useful_card = (1 << max_dimension_used) * 2;
    if next_index > max_useful_card {
        return;
    }
    if cards_to_add > 1 {
        let mut distances2 = distances.clone();
        let mut quads2 = quads;
        for i in 0..next_index {
            if (hand >> i) & 1 == 1 {
                // If there's a pair of cards in the hand with XOR x, and you
                // add a new card which has XOR x with some other card in the
                // hand, then those four cards form a quad. This counts each
                // quad three times, so we divide by three later.
                quads2 += distances2[(i ^ next_index) as usize] as u64;
                distances2[(i ^ next_index) as usize] += 1;
            }
        }
        // Try adding the card at `next_index`.
        iteration(
            hand | (1 << next_index),
            distances2,
            next_index + 1,
            max_index,
            cards_to_add - 1,
            quads2,
            best_quads,
            best_hand,
        );
        // You can assume the first 4 cards are selected since any optimal
        // solution contains a quad, and then you can assume the 5th and 6th
        // cards are selected because of symmetry.
        if next_index >= 6 {
            // Try not adding the card at `next_index`.
            iteration(
                hand,
                distances,
                next_index + 1,
                max_index,
                cards_to_add,
                quads,
                best_quads,
                best_hand,
            );
        }
    } else {
        // One card left to add, so try all possibilities.
        for j in next_index..max_index.min(max_useful_card + 1) {
            let mut quads2 = quads;
            for i in 0..next_index {
                if (hand >> i) & 1 == 1 {
                    quads2 += distances[(i ^ j) as usize] as u64;
                }
            }
            // Quads are triple-counted, so divide by 3.
            if quads2 / 3 > *best_quads {
                *best_quads = quads2 / 3;
                *best_hand = hand | (1 << j);
            }
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Number of cards in the hand (increase this slowly!)
    #[arg(default_value_t = 9)]
    cards_in_hand: u8,
    /// Number of cards in the deck (not necessarily a power of two, max 128)
    #[arg(default_value_t = 128)]
    cards_in_deck: u8,
}

fn main() {
    let Cli {
        cards_in_hand,
        cards_in_deck,
    } = Cli::parse();
    if cards_in_deck > 128 {
        println!("The maximum supported deck size is 128.");
        return;
    }
    if cards_in_hand > cards_in_deck {
        println!("The requested hand is bigger than the deck.");
        return;
    }
    let mut best_quads = 0;
    let mut best_hand = 0;
    let start = Instant::now();
    iteration(
        0,
        [0; 128],
        0,
        cards_in_deck,
        cards_in_hand,
        0,
        &mut best_quads,
        &mut best_hand,
    );
    println!("Time: {:?}", start.elapsed());
    println!("Max quads: {best_quads}");
    println!("Best hand:");
    for i in 0..8 {
        for j in 0..16 {
            print!("{}", (best_hand >> (i * 16 + j)) & 1);
        }
        println!("");
    }
}
