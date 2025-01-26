use std::{
    sync::{
        atomic::{AtomicI64, Ordering},
        Mutex,
    },
    thread,
    time::Instant,
};

use clap::{Parser, Subcommand};

/// Search for a hand.
///
/// `hand` is the current partial hand, represented as a bitvector.
/// `differences[i]` is the number of pairs of cards in the hand whose XOR is `i`.
/// `next_index` is the next card to (maybe) add.
/// `max_index` is the size of the deck.
/// `cards_in_hand` is the number of cards in the final hand.
/// `cards_to_add` is the number of cards left to add.
/// `quads` is the number of quads in the current partial hand times 3.
/// `target_quads` is the desired number of quads, if not searching for the
/// maximum.
/// `best_score` is the highest number of quads in a hand found so far when
/// searching for the maximum number of quads, or whether a valid hand has been
/// found when searching for a specific number of quads.
/// `best_table` is the hand that led to `best_score`.
///
/// Returns `None` if an obviously optimal solution has been found and
/// `Some(())` if not.
fn search_inner(
    hand: u128,
    differences: [u8; 128],
    next_index: u8,
    max_index: u8,
    cards_in_hand: u8,
    cards_to_add: u8,
    quads: u64,
    target_quads: Option<u64>,
    best_score: &mut u64,
    best_hand: &mut u128,
) -> Option<()> {
    // Nothing useful to do.
    if next_index + cards_to_add > max_index || cards_to_add == 0 {
        return Some(());
    }
    let last_card_in_hand = hand.checked_ilog2().unwrap_or(0);
    // The maximum dimension of the affine space used by a card in the hand,
    // indexed from 0.
    let max_dimension_used = last_card_in_hand.checked_ilog2().unwrap_or(0);
    // Don't allow adding a card that adds a new dimension to the hand unless
    // it's the first possible such card. (For example, if the highest card in
    // the hand is 3, there's no point adding any card above 4 since you could
    // equivalently add card 4 instead.)
    let max_useful_card = (1 << max_dimension_used) * 2;
    if next_index > max_useful_card {
        return Some(());
    }
    if cards_to_add > 1 {
        let mut differences2 = differences.clone();
        let mut quads2 = quads;
        let mut good = true;
        for i in 0..next_index {
            let difference = (i ^ next_index) as usize;
            if (hand >> i) & 1 == 1 {
                // If there's a pair of cards in the hand with XOR x, and you
                // add a new card which has XOR x with some other card in the
                // hand, then those four cards form a quad. This counts each
                // quad three times, so we divide by three later.
                quads2 += differences2[difference] as u64;
                if differences2[difference] > 1 {
                    // good = false;
                }
                differences2[difference] += 1;
            }
        }
        // Try adding the card at `next_index`, if that doesn't create too many
        // quads.
        // Note that `quads2` triple-counts quads, so we need to multiply
        // `target` by 3.
        if target_quads.is_none_or(|target| quads2 <= target * 3) && good {
            search_inner(
                hand | (1 << next_index),
                differences2,
                next_index + 1,
                max_index,
                cards_in_hand,
                cards_to_add - 1,
                quads2,
                target_quads,
                best_score,
                best_hand,
            )?;
        }
        let cards_in_hand2 = cards_in_hand as u64;
        // By symmetry, any solution for anything can be assumed to contain
        // the first 3 cards.
        // For everything except caps, you can assume the first 4 cards are
        // selected since any optimal solution contains a quad. Then, you can
        // assume the 5th card is selected because of symmetry.
        // If you're searching for hands with the maximum number of quads
        // (or just enough quads) you can assume the sixth card is selected as
        // well for more complicated reasons.
        let initial_segment_length = match target_quads {
            Some(0) => 3,
            Some(target) => {
                if target > (cards_in_hand2 * (cards_in_hand2 - 1)) / 2 {
                    6
                } else {
                    5
                }
            }
            None => 6,
        };
        if next_index >= initial_segment_length {
            // Try not adding the card at `next_index`.
            search_inner(
                hand,
                differences,
                next_index + 1,
                max_index,
                cards_in_hand,
                cards_to_add,
                quads,
                target_quads,
                best_score,
                best_hand,
            )?;
        }
    } else {
        // One card left to add, so try all possibilities.
        for j in next_index..max_index.min(max_useful_card + 1) {
            let mut quads2 = quads;
            for i in 0..next_index {
                if (hand >> i) & 1 == 1 {
                    quads2 += differences[(i ^ j) as usize] as u64;
                }
            }
            // Quads are triple-counted, so divide by 3.
            let real_quads = quads2 / 3;
            if let Some(target) = target_quads {
                if real_quads == target && *best_score == 0 {
                    *best_score = 1;
                    *best_hand = hand | (1 << j);
                }
            } else {
                if real_quads > *best_score {
                    *best_score = quads2 / 3;
                    *best_hand = hand | (1 << j);
                }
            }
        }
    };
    Some(())
}

/// Search for a hand.
///
/// `cards_in_deck` is the size of the deck.
/// `cards_in_hand` is the number of cards left to add.
/// `target_quads` is the desired number of quads, if not searching for the
/// maximum.
///
/// Returns the best hand and its score.
fn search(cards_in_deck: u8, cards_in_hand: u8, target_quads: Option<u64>) -> (u128, u64) {
    let mut best_hand = (1 << cards_in_hand) - 1;
    let mut best_score = 0;
    search_inner(
        0,
        [0; 128],
        0,
        cards_in_deck,
        cards_in_hand,
        cards_in_hand,
        0,
        target_quads,
        &mut best_score,
        &mut best_hand,
    );
    (best_hand, best_score)
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check one hand size
    #[command(arg_required_else_help = true)]
    Search {
        /// Number of cards in the hand (increase this slowly, start with 9)
        cards_in_hand: u8,
        /// Number of cards in the deck (max 128)
        #[arg(default_value_t = 128)]
        cards_in_deck: u8,
        /// Number of quads the hand should have. If not specified, searches for the maximum possible number of quads.
        target_quads: Option<u64>,
    },
    /// Check lots of hand sizes
    #[command(arg_required_else_help = true)]
    SearchAll {
        /// Initial hand size
        initial_cards_in_hand: u8,
        /// Number of cards in the deck (max 128)
        #[arg(default_value_t = 128)]
        cards_in_deck: u8,
    },
}

fn main() {
    match Cli::parse().command {
        Commands::Search {
            cards_in_hand,
            cards_in_deck,
            target_quads,
        } => {
            if cards_in_deck > 128 {
                println!("The maximum supported deck size is 128.");
                return;
            }
            if cards_in_hand > cards_in_deck {
                println!("The requested hand is bigger than the deck.");
                return;
            }
            let start = Instant::now();
            let (best_hand, best_score) = search(cards_in_deck, cards_in_hand, target_quads);
            println!("Time: {:?}", start.elapsed());
            if target_quads.is_none() {
                println!("Max quads: {best_score}");
            } else if best_score > 0 {
                println!("Found a hand.");
                if let Some(max_dim) = best_hand.checked_ilog2() {
                    println!("Max card used: {max_dim}");
                }
            } else {
                println!("No hand found.");
            }
            println!("Best hand:");
            for i in 0..8 {
                for j in 0..16 {
                    print!("{}", (best_hand >> (i * 16 + j)) & 1);
                }
                println!("");
            }
        }
        Commands::SearchAll {
            initial_cards_in_hand,
            cards_in_deck,
        } => {
            let threads = thread::available_parallelism()
                .map(|x| x.get())
                .unwrap_or(1);
            println!("Threads: {threads}");
            for cards_in_hand in initial_cards_in_hand..=cards_in_deck {
                println!("Hand size {cards_in_hand}, deck size {cards_in_deck}:");
                let (best_hand, best_score) = search(cards_in_deck, cards_in_hand, None);
                if let Some(max_card) = best_hand.checked_ilog2() {
                    println!("{best_score} quads with max card {max_card}");
                } else {
                    println!("{best_score} quads");
                }
                let target = AtomicI64::new((best_score - 1) as i64);
                let results = Mutex::new(vec![]);
                thread::scope(|s| {
                    for _tid in 0..threads {
                        s.spawn(|| loop {
                            let j = target.fetch_sub(1, Ordering::Relaxed);
                            if j < 0 {
                                break;
                            }
                            let (best_hand, best_score) =
                                search(cards_in_deck, cards_in_hand, Some(j as u64));
                            if best_score > 0 {
                                results
                                    .lock()
                                    .unwrap()
                                    .push((j, best_hand.checked_ilog2().unwrap()));
                            }
                        });
                    }
                });
                let mut results = results.into_inner().unwrap();
                results.sort_by_key(|x| -x.0);
                for (j, max) in results {
                    println!("{j} quads with max card {max}");
                }
            }
        }
    }
}
