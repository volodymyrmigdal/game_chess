#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

//!
//! Implement mechanics of the game chess.
//!

use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
pub use pleco::
{
    core::Player,
    core::PieceType,
    core::Piece,
    board::piece_locations::PieceLocations, //Minimal board impl

    core::piece_move::MoveType,
    core::piece_move::BitMove as Move, //https://docs.rs/pleco/latest/pleco/core/piece_move/index.html
    core::move_list::MoveList,
    core::sq::SQ as Cell,
    core::bitboard::BitBoard as CellsSet,
};

use serde::
{
    Serialize,
    Deserialize,
    Serializer,
    Deserializer,
};

/* Structure:

Board
  pleco_board : pleco::Board

HistoryEntry
  fen : String
  uci_move : String

Game
   board : Board
   history : Vec<HistoryEntry>
*/

/* List of resources to show

  tools::eval::Eval,
  helper::Helper,
  helper::prelude,
  bot_prelude

*/

const SAVES_FOLDER_NAME: &str = "saves";
const SAVE_FILE_EXTENSION: &str = ".save";

///
/// Game board
///

#[derive(Debug)]
pub struct Board
{
    pleco_board: pleco::Board,
}

impl Board
{
    ///
    /// Constructs a board with the starting position
    ///
    pub fn default() -> Self
    {
        Self
        {
            pleco_board: pleco::Board::start_pos()
        }
    }

    ///
    /// Constructs a aborad from FEN
    ///
    pub fn from_fen(fen: &Fen) -> Self
    {
        match pleco::Board::from_fen(&fen)
        {
            Ok(pleco_board) => Self { pleco_board },
            _ => Self::default()
        }
    }

    ///
    /// Makes move on the board. Accepts move in UCI format.
    ///
    pub fn make_move(&mut self, uci_move: &str) -> Option<Self>
    {
        let mut pleco_board: pleco::Board = self.pleco_board.clone();
        let result = pleco_board.apply_uci_move(&uci_move);
        if result
        {
            Some(Self { pleco_board })
        } else {
            None
        }
    }

    ///
    /// Checks if the move is valid. Accepts move in UCI format.
    ///
    pub fn move_is_valid(&self, uci_move: &str) -> bool
    {
        match self.move_from_uci(uci_move)
        {
            Some(m) => self.pleco_board.pseudo_legal_move(m) && self.pleco_board.legal_move(m),
            _ => false
        }
    }

    ///
    /// Makes [Move] from move in UCI format.
    ///
    pub fn move_from_uci(&self, uci_move: &str) -> Option<Move>
    {
        let all_moves: MoveList = self.pleco_board.generate_moves();
        all_moves.iter()
            .find(|m| m.stringify() == uci_move)
            .cloned()
    }

    ///
    /// Evaluates the score of a [Board] for the current side to move.
    ///
    pub fn score(&self) -> i32
    {
        0
        /* ttt : implement me */
    }

    ///
    /// True if the current side to move is in check mate.
    ///
    pub fn is_checkmate(&self) -> bool
    {
        self.pleco_board.checkmate()
    }

    ///
    /// Is the current side to move is in stalemate.
    ///
    pub fn is_stalemate(&self) -> bool
    {
        self.pleco_board.stalemate()
    }

    ///
    /// Return the `Player` whose turn it is to move.
    ///
    pub fn current_turn(&self) -> Player
    {
        self.pleco_board.turn()
    }

    ///
    /// Return the last move played, if any.
    ///
    pub fn last_move(&self) -> Option<Move>
    {
        self.pleco_board.last_move()
    }

    ///
    /// Returns pretty-printed string representation of the board
    ///
    pub fn to_pretty_string(&self) -> String
    {
        let mut s = String::with_capacity(pleco::core::masks::SQ_CNT * 2 + 40);
        let mut rank = 8;

        for sq in pleco::core::masks::SQ_DISPLAY_ORDER.iter()
        {
        if sq % 8 == 0
        {
            s.push(char::from_digit(rank, 10).unwrap());
            s.push_str(" | ");
            rank -= 1;
        }

        let op = self.pleco_board.get_piece_locations().piece_at(pleco::SQ(*sq));
        let char = if op != Piece::None { op.character_lossy() } else { '-' };
        s.push(char);
        s.push(' ');

        if sq % 8 == 7
        {
            s.push('\n');
        }
        }

        s.push_str("  ------------------\n");
        s.push_str("    a b c d e f g h");

        s
    }

    ///
    /// Prints board to the terminal.
    ///
    pub fn print(&self) /* qqq : remove. instead return string */
    {
        println!("{}", self.to_pretty_string());
    }

    ///
    /// Creates a 'Fen` string of the board.
    ///
    pub fn to_fen(&self) -> Fen
    {
        self.pleco_board.fen()
    }
}

///
///Positions on the board in [FEN](https://www.chess.com/terms/fen-chess#what-is-fen) format
///

pub type Fen = String;

///
/// Contains information about move made in the past.
/// Field `fen` contains representation of the board as FEN string
/// Field `uci_move` contains move in UCI format
///

#[derive(Serialize, Deserialize, Debug)]
pub struct HistoryEntry
{
    fen: Fen,
    uci_move: String,
}

///
/// Status of the game
///

#[derive(Debug, PartialEq)]
pub enum GameStatus
{
    /// The game is not finished, and the game is still in play.
    Continuing,
    /// The game has the winner.
    Checkmate,
    /// The game is drawn.
    Stalemate,
}

///
/// Interface for playing chess game.
///
/// Basically Board + History.
///

#[derive(Serialize, Deserialize, Debug)]
pub struct Game
{
    #[serde(serialize_with = "board_ser", deserialize_with = "board_der")]
    board: Board,
    history: Vec<HistoryEntry>,
    date: SystemTime, // unix timestamp
}

impl Game
{
    ///
    /// Constructs a new game with default board setup
    ///
    pub fn default() -> Self
    {
        Self
        {
            board: Board::default(),
            history: Vec::new(),
            date: SystemTime::now(),
        }
    }
    /* xxx : ? */

    ///
    /// Makes a move on the board. Accepts move in UCI format. For example, "e2e4".
    /// Updates histort and returns `true` if move was succesfuly applied, otherwise returns `false`.
    /// The board and history are not changed in case of fail.
    ///
    pub fn make_move(&mut self, uci_move: &str) -> bool
    {
        let new_board = self.board.make_move(uci_move);
        let success = !new_board.is_none();
        if success
        {
            self.board = new_board.unwrap();
            self.history.push(HistoryEntry { fen: self.board.to_fen(), uci_move: uci_move.to_string() });
        }
        success
    }

    ///
    /// Return the [Player] whose turn it is to move.
    ///
    pub fn current_turn(&self) -> Player
    {
        self.board.current_turn()
    }

    ///
    /// Prints board to the terminal.
    ///
    pub fn board_print(&self)
    {
        self.board.print();
    }

    ///
    /// Returns current game status as [GameStatus].
    ///
    pub fn status(&self) -> GameStatus
    {
        if self.board.is_checkmate()
        {
            return GameStatus::Checkmate;
        }

        if self.board.is_stalemate()
        {
            return GameStatus::Stalemate;
        }

        return GameStatus::Continuing;
    }

    ///
    /// Returns last move as UCI string. For example: "a2a4"
    /// Returns None if there are no moves.
    ///
    pub fn last_move(&self) -> Option<String>
    {
        match self.history.last()
        {
            Some(h) => Some(h.uci_move.clone()),
            _ => None
        }
    }

    ///
    /// Saves game to file
    ///
    pub fn save(&self) -> std::io::Result<String> {
        fs::create_dir_all(SAVES_FOLDER_NAME)?;

        let serialized =  serde_json::to_string( &self ).unwrap();
        let file_id = get_unix_timestamp(None);
        let filename = format!("{}/{}{}", SAVES_FOLDER_NAME, file_id.to_string(), SAVE_FILE_EXTENSION);
        let filepath = Path::new(&filename);

        let mut file = File::create(filepath).unwrap();

        match file.write_all(serialized.as_bytes()) {
            Ok(_) => Ok(filename),
            Err(error) => Err(error)
        }
    }
}

///
/// Get unix timestamp in seconds.
///

pub fn get_unix_timestamp(start: Option<SystemTime>) -> u64 {
    let start = start.unwrap_or(SystemTime::now());
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    since_the_epoch.as_secs()
}

///
/// Serialize game to string.
///

pub fn board_ser<S: Serializer>(board: &Board, s: S) -> Result<S::Ok, S::Error>
{
    s.serialize_str(&board.to_fen())
}

///
/// Deserialize game from string to FEN and make board.
///

pub fn board_der<'de, D: Deserializer<'de>>(d: D) -> Result<Board, D::Error>
{
  let fen : String = Deserialize::deserialize( d )?;
  Ok( Board::from_fen( &fen ) )
}
