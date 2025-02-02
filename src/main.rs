use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread,
    time::Instant,
};

use clap::{Parser, Subcommand};

/// Search for a hand with a single target quad count.
///
/// `hand` is the current partial hand, represented as a bitvector.
/// `differences[i]` is the number of pairs of cards in the hand whose XOR is `i`.
/// `min_diff_count` is the minimum allowed second entry in `differences`.
/// `max_diff_count` is the maximum allowed entry in `differences`.
/// `next_index` is the next card to (maybe) add.
/// `max_index` is the size of the deck.
/// `cards_in_hand` is the number of cards in the final hand.
/// `cards_to_add` is the number of cards left to add.
/// `quads` is the number of quads in the current partial hand times 3.
/// `target_quads` is the desired number of quads, if not searching for the
/// maximum.
/// `best_score` is the highest number of quads in a hand found so far when
/// searching for the maximum number of quads, or the lowest max card when
/// searching for a specific number of quads.
/// `best_table` is the hand that led to `best_score`.
///
/// Returns `None` if searching for a specific number of quads and that has
/// been achieved, and `Some(())` otherwise.
fn search_inner(
    hand: u128,
    differences: [u8; 128],
    min_diff_count: usize,
    max_diff_count: usize,
    next_index: usize,
    max_index: usize,
    cards_in_hand: usize,
    cards_to_add: usize,
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
            let difference = i ^ next_index;
            if (hand >> i) & 1 == 1 {
                // If there's a pair of cards in the hand with XOR x, and you
                // add a new card which has XOR x with some other card in the
                // hand, then those four cards form a quad. This counts each
                // quad three times, so we divide by three later.
                quads2 += differences2[difference] as u64;
                differences2[difference] += 1;
                // Don't try adding this card if that would violate max_diff_count.
                if differences2[difference] as usize > max_diff_count {
                    good = false;
                    break;
                }
            }
        }
        // Try adding the card at `next_index`, if that doesn't create too many
        // quads.
        // Note that `quads2` triple-counts quads, so we need to multiply
        // `target` by 3.
        if good && target_quads.is_none_or(|target| quads2 <= target * 3) {
            search_inner(
                hand | (1 << next_index),
                differences2,
                min_diff_count,
                max_diff_count,
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
        if next_index >= min_diff_count * 2 {
            // Try not adding the card at `next_index`.
            search_inner(
                hand,
                differences,
                min_diff_count,
                max_diff_count,
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
                    quads2 += differences[i ^ j] as u64;
                }
            }
            // Quads are triple-counted, so divide by 3.
            let real_quads = quads2 / 3;
            if let Some(target) = target_quads {
                let j2 = j as u64;
                if real_quads == target && j2 < *best_score {
                    *best_score = j2;
                    *best_hand = hand | (1 << j);
                }
                // Exit early, since we found a solution.
                return None;
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

/// Search for a hand with lots of target quad counts.
///
/// `hand` is the current partial hand, represented as a bitvector.
/// `differences[i]` is the number of pairs of cards in the hand whose XOR is `i`.
/// `min_diff_count` is the minimum allowed second entry in `differences`.
/// `max_diff_count` is the maximum allowed entry in `differences`.
/// `next_index` is the next card to (maybe) add.
/// `max_index` is the size of the deck.
/// `cards_in_hand` is the number of cards in the final hand.
/// `cards_to_add` is the number of cards left to add.
/// `quads` is the number of quads in the current partial hand times 3.
/// `best_scores` contains the lowest max card for each quad count.
fn search_inner_multi(
    hand: u128,
    differences: [u8; 128],
    min_diff_count: usize,
    max_diff_count: usize,
    next_index: usize,
    max_index: usize,
    cards_in_hand: usize,
    cards_to_add: usize,
    quads: u64,
    best_scores: &mut Vec<u64>,
) {
    // Nothing useful to do.
    if next_index + cards_to_add > max_index || cards_to_add == 0 {
        return;
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
        return;
    }
    if cards_to_add > 1 {
        let mut differences2 = differences.clone();
        let mut quads2 = quads;
        let mut good = true;
        for i in 0..next_index {
            let difference = i ^ next_index;
            if (hand >> i) & 1 == 1 {
                // If there's a pair of cards in the hand with XOR x, and you
                // add a new card which has XOR x with some other card in the
                // hand, then those four cards form a quad. This counts each
                // quad three times, so we divide by three later.
                quads2 += differences2[difference] as u64;
                differences2[difference] += 1;
                // Don't try adding this card if that would violate max_diff_count.
                if differences2[difference] as usize > max_diff_count {
                    good = false;
                    break;
                }
            }
        }
        // Try adding the card at `next_index`.
        if good {
            search_inner_multi(
                hand | (1 << next_index),
                differences2,
                min_diff_count,
                max_diff_count,
                next_index + 1,
                max_index,
                cards_in_hand,
                cards_to_add - 1,
                quads2,
                best_scores,
            );
        }
        if next_index >= min_diff_count * 2 {
            // Try not adding the card at `next_index`.
            search_inner_multi(
                hand,
                differences,
                min_diff_count,
                max_diff_count,
                next_index + 1,
                max_index,
                cards_in_hand,
                cards_to_add,
                quads,
                best_scores,
            );
        }
    } else {
        // One card left to add, so try all possibilities.
        for j in next_index..max_index.min(max_useful_card + 1) {
            let mut quads2 = quads;
            for i in 0..next_index {
                if (hand >> i) & 1 == 1 {
                    quads2 += differences[i ^ j] as u64;
                }
            }
            // Quads are triple-counted, so divide by 3.
            let real_quads = (quads2 / 3) as usize;
            // Don't overflow.
            if real_quads >= best_scores.len() {
                best_scores.extend((0..=real_quads - best_scores.len()).map(|_| max_index as u64));
            }
            if best_scores[real_quads] > j as u64 {
                best_scores[real_quads] = j as u64;
            }
        }
    };
}

/// Search for a hand.
///
/// `cards_in_deck` is the size of the deck.
/// `cards_in_hand` is the size of the target hand.
/// `target_quads` is the desired number of quads, if not searching for the
/// maximum.
///
/// Returns the best hand and its score.
fn search(cards_in_deck: usize, cards_in_hand: usize, target_quads: Option<u64>) -> (u128, u64) {
    let mut best_hand = (1 << cards_in_hand) - 1;
    let mut best_score = if target_quads == None {
        0
    } else {
        cards_in_deck as u64
    };
    // let min_max_diff_count = match target_quads {
    //     None => 3,
    //     Some(target) => {
    //         if target > (cards_in_hand * (cards_in_hand + 1) / 12) as u64 {
    //             3
    //         } else if target > 0 {
    //             2
    //         } else {
    //             1
    //         }
    //     }
    // }
    // .min(cards_in_hand / 2);
    // for max_diff_count in min_max_diff_count..=(cards_in_hand / 2) {
    //     search_inner(
    //         0,
    //         [0; 128],
    //         max_diff_count,
    //         0,
    //         cards_in_deck,
    //         cards_in_hand,
    //         cards_in_hand,
    //         0,
    //         target_quads,
    //         &mut best_score,
    //         &mut best_hand,
    //     );
    // }
    if let Some(target) = target_quads {
        if target == 0 {
            search_inner(
                0,
                [0; 128],
                1,
                1,
                0,
                cards_in_deck,
                cards_in_hand,
                cards_in_hand,
                0,
                target_quads,
                &mut best_score,
                &mut best_hand,
            );
        }
        if target <= (cards_in_hand * (cards_in_hand + 1) / 12) as u64 {
            search_inner(
                0,
                [0; 128],
                2,
                2,
                0,
                cards_in_deck,
                cards_in_hand,
                cards_in_hand,
                0,
                target_quads,
                &mut best_score,
                &mut best_hand,
            );
        }
    }
    search_inner(
        0,
        [0; 128],
        3,
        cards_in_deck / 2,
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

/// Search for many hands.
///
/// `cards_in_deck` is the size of the deck.
/// `cards_in_hand` is the size of the target hands.
///
/// The `n`th element of the result is the maximum card used in a hand with `n`
/// quads if one exists, or `cards_in_deck` otherwise.
fn search_multi(mut cards_in_deck: usize, cards_in_hand: usize) -> Vec<u64> {
    let mut ret = vec![];
    while cards_in_deck > 0 && cards_in_deck >= cards_in_hand {
        search_inner_multi(
            0,
            [0; 128],
            1,
            1,
            0,
            cards_in_deck,
            0,
            cards_in_hand,
            0,
            &mut ret,
        );
        search_inner_multi(
            0,
            [0; 128],
            2,
            2,
            0,
            cards_in_deck,
            0,
            cards_in_hand,
            0,
            &mut ret,
        );
        search_inner_multi(
            0,
            [0; 128],
            3,
            cards_in_deck / 2,
            0,
            cards_in_deck,
            0,
            cards_in_hand,
            0,
            &mut ret,
        );
        // Make sure we don't miss a solution with lower dimension than the
        // first one
        cards_in_deck /= 2;
    }
    ret
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
        cards_in_hand: usize,
        /// Number of cards in the deck (max 128)
        #[arg(default_value_t = 128)]
        cards_in_deck: usize,
        /// Number of quads the hand should have. If not specified, searches for the maximum possible number of quads.
        target_quads: Option<u64>,
    },
    /// Check lots of hand sizes
    #[command(arg_required_else_help = true)]
    SearchAll {
        /// Initial hand size
        initial_cards_in_hand: usize,
        /// Number of cards in the deck (max 128)
        #[arg(default_value_t = 128)]
        cards_in_deck: usize,
        /// Maximum hand size
        max_cards_in_hand: Option<usize>,
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
            max_cards_in_hand,
        } => {
            let max_cards_in_hand = max_cards_in_hand.unwrap_or(cards_in_deck);
            let threads = thread::available_parallelism()
                .map(|x| x.get())
                .unwrap_or(1);
            println!("Threads: {threads}");
            let cards_in_hand_atomic = AtomicUsize::new(initial_cards_in_hand);
            let print_guard = Mutex::new(());
            thread::scope(|s| {
                for _tid in 0..threads {
                    s.spawn(|| loop {
                        let cards_in_hand = cards_in_hand_atomic.fetch_add(1, Ordering::Relaxed);
                        if cards_in_hand > max_cards_in_hand {
                            break;
                        }
                        let res = search_multi(cards_in_deck, cards_in_hand);
                        let guard = print_guard.lock().unwrap();
                        println!("Hand size {cards_in_hand}, deck size {cards_in_deck}:");
                        for (j, &max) in res.iter().enumerate() {
                            if max < cards_in_deck as u64 {
                                println!("{j} quads with max card {max}");
                            }
                        }
                        drop(guard);
                    });
                }
            })
        }
    }
}
