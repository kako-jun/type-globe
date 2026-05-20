use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Ta25SeatColor {
    Red,
    Blue,
    Green,
    Yellow,
}

impl Ta25SeatColor {
    pub const ALL: [Self; 4] = [Self::Red, Self::Blue, Self::Green, Self::Yellow];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Yellow => "yellow",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Ta25SeatKind {
    Human,
    Cpu,
    Empty,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Ta25Seat {
    pub color: Ta25SeatColor,
    pub kind: Ta25SeatKind,
    /// Stable logical participant ID. In local prototypes this can be a
    /// simple internal token; online play can later map it to a remote ID.
    pub player_id: String,
    pub display_name: String,
}

impl Ta25Seat {
    fn human(color: Ta25SeatColor, player_id: &str, display_name: &str) -> Self {
        Self {
            color,
            kind: Ta25SeatKind::Human,
            player_id: player_id.to_string(),
            display_name: display_name.to_string(),
        }
    }

    fn cpu(color: Ta25SeatColor, ordinal: usize) -> Self {
        Self {
            color,
            kind: Ta25SeatKind::Cpu,
            player_id: format!("cpu-{}", color.as_str()),
            display_name: format!("CPU {ordinal}"),
        }
    }

    fn empty(color: Ta25SeatColor) -> Self {
        Self {
            color,
            kind: Ta25SeatKind::Empty,
            // Sentinel only, not a real stable participant identity.
            player_id: format!("empty-{}", color.as_str()),
            display_name: "(empty)".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Ta25Roster {
    pub seats: [Ta25Seat; 4],
}

impl Ta25Roster {
    /// Explicit empty four-seat roster. Reserved for future seat-claim
    /// flows; the current `add_human()` API intentionally models only the
    /// local prototype path (red stays local starter, then blue/green/yellow
    /// are replaced in order).
    pub fn all_empty() -> Self {
        Self {
            seats: std::array::from_fn(|index| Ta25Seat::empty(Ta25SeatColor::ALL[index])),
        }
    }

    /// Canonical local prototype setup for TA25:
    /// one human seat plus three CPU seats.
    pub fn standard_local(human_name: &str) -> Self {
        let mut roster = Self::all_empty();
        roster.seats[0] = Ta25Seat::human(Ta25SeatColor::Red, "local-human", human_name);
        roster.seats[1] = Ta25Seat::cpu(Ta25SeatColor::Blue, 1);
        roster.seats[2] = Ta25Seat::cpu(Ta25SeatColor::Green, 2);
        roster.seats[3] = Ta25Seat::cpu(Ta25SeatColor::Yellow, 3);
        roster
    }

    /// Canonical join path for extra humans. The seat-allocation rule is:
    /// red stays the local starter, then blue -> green -> yellow are replaced
    /// in that order as humans join. Duplicate player IDs are rejected.
    #[allow(dead_code)]
    pub fn add_human(&mut self, player_id: &str, display_name: &str) -> bool {
        if self
            .seats
            .iter()
            .any(|seat| seat.kind == Ta25SeatKind::Human && seat.player_id == player_id)
        {
            return false;
        }

        let Some(seat) = self
            .seats
            .iter_mut()
            .skip(1)
            .find(|seat| seat.kind != Ta25SeatKind::Human)
        else {
            return false;
        };
        *seat = Ta25Seat::human(seat.color, player_id, display_name);
        true
    }

    pub fn summary_line(&self) -> String {
        self.seats
            .iter()
            .map(|seat| {
                format!(
                    "{}={}({})",
                    seat.color.as_str(),
                    seat.display_name,
                    seat.kind.as_str()
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Ta25SeatKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Cpu => "cpu",
            Self::Empty => "empty",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_local_is_one_human_plus_three_cpu() {
        let roster = Ta25Roster::standard_local("You");
        assert_eq!(roster.seats[0].color, Ta25SeatColor::Red);
        assert_eq!(roster.seats[0].kind, Ta25SeatKind::Human);
        assert_eq!(roster.seats[0].display_name, "You");

        let cpu_kinds = roster
            .seats
            .iter()
            .skip(1)
            .map(|seat| seat.kind)
            .collect::<Vec<_>>();
        assert_eq!(
            cpu_kinds,
            vec![Ta25SeatKind::Cpu, Ta25SeatKind::Cpu, Ta25SeatKind::Cpu]
        );
    }

    #[test]
    fn standard_local_uses_all_four_colors_once() {
        let roster = Ta25Roster::standard_local("You");
        let colors = roster
            .seats
            .iter()
            .map(|seat| seat.color)
            .collect::<Vec<_>>();
        assert_eq!(colors, Ta25SeatColor::ALL);
    }

    #[test]
    fn adding_human_replaces_next_cpu_in_canonical_order() {
        let mut roster = Ta25Roster::standard_local("You");
        assert!(roster.add_human("guest-1", "Guest 1"));
        let blue = roster
            .seats
            .iter()
            .find(|seat| seat.color == Ta25SeatColor::Blue)
            .expect("blue seat");
        assert_eq!(blue.kind, Ta25SeatKind::Human);
        assert_eq!(blue.player_id, "guest-1");
        assert_eq!(blue.display_name, "Guest 1");
    }

    #[test]
    fn duplicate_human_id_is_rejected() {
        let mut roster = Ta25Roster::standard_local("You");
        assert!(!roster.add_human("local-human", "Guest 1"));
    }

    #[test]
    fn adding_humans_consumes_blue_green_yellow_in_order() {
        let mut roster = Ta25Roster::standard_local("You");
        assert!(roster.add_human("guest-1", "Guest 1"));
        assert!(roster.add_human("guest-2", "Guest 2"));
        assert!(roster.add_human("guest-3", "Guest 3"));
        assert!(!roster.add_human("guest-4", "Guest 4"));

        let humans = roster
            .seats
            .iter()
            .filter(|seat| seat.kind == Ta25SeatKind::Human)
            .map(|seat| (seat.color, seat.player_id.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            humans,
            vec![
                (Ta25SeatColor::Red, "local-human"),
                (Ta25SeatColor::Blue, "guest-1"),
                (Ta25SeatColor::Green, "guest-2"),
                (Ta25SeatColor::Yellow, "guest-3"),
            ]
        );
    }

    #[test]
    fn all_empty_exposes_empty_seats() {
        let roster = Ta25Roster::all_empty();
        assert!(roster
            .seats
            .iter()
            .all(|seat| seat.kind == Ta25SeatKind::Empty));
    }

    #[test]
    fn summary_line_mentions_each_seat() {
        let roster = Ta25Roster::standard_local("You");
        assert_eq!(
            roster.summary_line(),
            "red=You(human), blue=CPU 1(cpu), green=CPU 2(cpu), yellow=CPU 3(cpu)"
        );
    }
}
