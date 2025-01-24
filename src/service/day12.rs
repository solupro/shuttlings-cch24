use actix_web::{web, HttpResponse};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::str::FromStr;
use std::sync::Arc;
use std::{fmt, sync};

const BOARD_WIDTH: usize = 4;
const BOARD_HEIGHT: usize = 4;

#[derive(Default, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
enum Cell {
    #[default]
    Empty,
    Cookie,
    Milk,
}

#[derive(Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum Column {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
}

impl From<bool> for Cell {
    fn from(b: bool) -> Self {
        if b {
            Cell::Cookie
        } else {
            Cell::Milk
        }
    }
}

impl FromStr for Cell {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cookie" => Ok(Cell::Cookie),
            "empty" => Ok(Cell::Empty),
            "milk" => Ok(Cell::Milk),
            _ => Err(format!("Invalid cell: {}", s)),
        }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Cell::Empty => "⬛",
            Cell::Cookie => "🍪",
            Cell::Milk => "🥛",
        })
    }
}

pub struct Board {
    rows: [[Cell; BOARD_WIDTH]; BOARD_HEIGHT],
    rng: StdRng,
}

impl Board {
    fn winner(&self) -> Option<Cell> {
        if self.is_win(&Cell::Cookie) {
            return Some(Cell::Cookie);
        }

        if self.is_win(&Cell::Milk) {
            return Some(Cell::Milk);
        }

        if (0..BOARD_WIDTH).any(|i| self.rows[i].iter().any(|j| j == &Cell::Empty)) {
            return Some(Cell::Empty);
        }

        None
    }

    fn is_win(&self, cell: &Cell) -> bool {
        (0..BOARD_WIDTH).any(|i| {
            self.rows[i].iter().all(|j| j == cell) || self.rows.iter().all(|j| j[i] == *cell)
        }) || (0..BOARD_WIDTH).all(|i| self.rows[i][i] == *cell)
            || (0..BOARD_WIDTH).all(|i| self.rows[i][3 - i] == *cell)
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            rows: Default::default(),
            rng: StdRng::seed_from_u64(2024),
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in 0..BOARD_WIDTH {
            writeln!(
                f,
                "⬜{}⬜",
                self.rows[row]
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<String>()
            )?;
        }

        write!(
            f,
            "⬜⬜⬜⬜⬜⬜\n{}",
            match self.winner() {
                Some(Cell::Milk) => "🥛 wins!\n",
                Some(Cell::Cookie) => "🍪 wins!\n",
                None => "No winner.\n",
                _ => "",
            }
        )
    }
}

async fn board(board: web::Data<Arc<sync::RwLock<Board>>>) -> String {
    board.read().unwrap().to_string()
}

async fn reset(b: web::Data<Arc<sync::RwLock<Board>>>) -> String {
    let mut board = b.write().unwrap();
    *board = Default::default();
    board.to_string()
}

async fn place(
    p: web::Path<(String, String)>,
    b: web::Data<Arc<sync::RwLock<Board>>>,
) -> HttpResponse {
    let (cell, column) = p.into_inner();

    // 解析 cell
    let cell = match Cell::from_str(&cell) {
        Ok(cell) if matches!(cell, Cell::Cookie | Cell::Milk) => cell,
        _ => return HttpResponse::BadRequest().body("response body does not matter"),
    };

    // 解析 column
    let column = match column.parse::<i32>() {
        Ok(cc) if (1..=4).contains(&cc) => cc,
        _ => return HttpResponse::BadRequest().body("response body does not matter"),
    };

    let board = b.read().unwrap();
    match board.winner() {
        Some(Cell::Cookie) | Some(Cell::Milk) | None => {
            return HttpResponse::ServiceUnavailable().body(board.to_string());
        }
        _ => {}
    }
    for row in (0..BOARD_HEIGHT).rev() {
        if board.rows[row][column as usize - 1] == Cell::Empty {
            drop(board);
            let mut board = b.write().unwrap();
            board.rows[row][column as usize - 1] = cell;
            return HttpResponse::Ok().body(board.to_string());
        }
    }

    HttpResponse::ServiceUnavailable().body(board.to_string())
}

async fn random_board(b: web::Data<Arc<sync::RwLock<Board>>>) -> HttpResponse {
    let mut board = b.write().unwrap();
    for i in 0..BOARD_HEIGHT {
        for j in 0..BOARD_WIDTH {
            board.rows[i][j] = board.rng.gen::<bool>().into();
        }
    }

    HttpResponse::Ok().body(board.to_string())
}

pub(crate) fn scope() -> actix_web::Scope {
    web::scope("12")
        .route("/board", web::get().to(board))
        .route("/reset", web::post().to(reset))
        .route("/place/{cell}/{column}", web::post().to(place))
        .route("/random-board", web::get().to(random_board))
}
