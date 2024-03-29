use crate::{
    game_type::{GSADataTaken, GSAResult, Game},
    mpmc::BroadcastChannel,
};
use cardpack::{Pack, Pile};
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug)]
pub struct Cheat {
    client_no_turn: Mutex<usize>,
    num_players: Mutex<usize>,

    game_has_started: Mutex<bool>,

    dealer_pile: Arc<Mutex<Pile>>,
    last_player_cheated: Arc<Mutex<bool>>,
}

impl Default for Cheat {
    fn default() -> Self {
        Self {
            num_players: Mutex::new(0),
            client_no_turn: Mutex::new(0),
            game_has_started: Mutex::new(false),
            dealer_pile: Arc::new(Mutex::new(Pile::default())),
            last_player_cheated: Arc::new(Mutex::new(false)),
        }
    }
}

impl Game for Cheat {
    const MIN_PLAYERS: usize = 3;
    // const MIN_PLAYERS: usize = 3;
    const GSAS: usize = 3;
    const DEALER_PILE_HIDDEN: bool = true;

    type GSA1Params = Pile;
    type GSA2Params = Pile;
    type GSA3Params = ();
    type GSA4Params = ();
    type GSA5Params = ();

    fn subscribe(&self) -> Option<usize> {
        if *self.game_has_started.lock() {
            None
        } else {
            let mut current_players = self.num_players.lock();
            *current_players += 1;
            Some(*current_players - 1)
        }
    }

    fn start(&self, broadcast_channel: Arc<BroadcastChannel<(Vec<Pile>, bool)>>) -> Option<Pile> {
        let n_players = *self.num_players.lock();
        if n_players >= Self::MIN_PLAYERS && !*self.game_has_started.lock() {
            *self.game_has_started.lock() = true;
            let mut deck = Pack::french_deck().cards().shuffle();

            let cards_per_person = (52 - (52 % n_players)) / n_players;
            let mut piles = vec![];
            for _ in 0..n_players {
                piles.push(deck.draw(cards_per_person).unwrap());
            }
            broadcast_channel.send((piles, true)).unwrap();

        }
    
        None
    }

    fn arc_dealer_pile(&self) -> Arc<Mutex<Pile>> {
        self.dealer_pile.clone()
    }

    fn has_started(&self) -> bool {
        *self.game_has_started.lock()
    }

    fn gsa_names_static() -> &'static [(&'static str, GSADataTaken)] {
        &[
            ("Add cards to pile", GSADataTaken::TakeCards),
            ("Cheat Add cards to pile", GSADataTaken::TakeCards),
            ("Call Cheat", GSADataTaken::Nothing),
        ]
    }

    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation
    )]
    fn last_player_id(&self) -> usize {
        ((*self.client_no_turn.lock() as i32 - 1) % *self.num_players.lock() as i32)
        as usize
    }

    ///Add cards to pile
    fn gsa_1(&self, _: usize, cards_to_add: Self::GSA1Params) -> GSAResult {
        *self.client_no_turn.lock() += 1;
        *self.client_no_turn.lock() %= *self.num_players.lock();

        self.dealer_pile.lock().append(&cards_to_add);
        GSAResult::Nothing
    }

    ///do the Cheat
    fn gsa_2(&self, _: usize, cards_to_add: Self::GSA2Params) -> GSAResult {
        *self.client_no_turn.lock() += 1;
        *self.client_no_turn.lock() %= *self.num_players.lock();

        self.dealer_pile.lock().append(&cards_to_add);
        *self.last_player_cheated.lock() = true;
        GSAResult::Nothing
    }

    ///call the Cheat
    fn gsa_3(&self, caller_id: usize, _: Self::GSA3Params) -> GSAResult {
        let pile = std::mem::take(&mut *self.dealer_pile.lock());
        if *self.last_player_cheated.lock() {
            GSAResult::PlayerTakesAllCards(
                pile, self.last_player_id()
            )
        } else {
            GSAResult::PlayerTakesAllCards(pile, caller_id)
        }
    }

    fn gsas_fufilled(&self, caller_id: usize) -> u8 {
        if *self.game_has_started.lock() {
            let mut res = 0;

            if !self.dealer_pile.lock().is_empty() && caller_id != self.last_player_id() { 
                res += 0b0010_0000;
            }
            if caller_id == *self.client_no_turn.lock() {
                res += 0b1100_0000;
            }

            res
        } else {
            0
        }
    }
}
