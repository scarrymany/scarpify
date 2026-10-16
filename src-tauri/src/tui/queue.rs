//! Play order, mirroring the window's player: shuffle keeps the original order to restore it,
//! repeat cycles off, all and one, and "previous" restarts a track that has played a while.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{Provider, Track};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

pub type TrackKey = (Provider, String);

pub fn key_of(track: &Track) -> TrackKey {
    (track.provider, track.id.clone())
}

#[derive(Default)]
pub struct Queue {
    tracks: Vec<Track>,
    index: Option<usize>,
    /// Order before shuffling, restored when shuffle is turned off.
    ordered: Vec<Track>,
    pub shuffle: bool,
    pub repeat: Repeat,
    rng: Rng,
}

impl Queue {
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn index(&self) -> Option<usize> {
        self.index
    }

    pub fn current(&self) -> Option<&Track> {
        self.index.and_then(|index| self.tracks.get(index))
    }

    pub fn upcoming(&self) -> &[Track] {
        match self.index {
            Some(index) => &self.tracks[index + 1..],
            None => &[],
        }
    }

    /// Returns the track to load.
    pub fn play(&mut self, tracks: &[Track], start: usize) -> Option<Track> {
        let first = tracks.get(start)?.clone();
        self.ordered = tracks.to_vec();
        if self.shuffle {
            let mut rest = without(tracks, start);
            self.rng.shuffle(&mut rest);
            self.tracks = std::iter::once(first.clone()).chain(rest).collect();
            self.index = Some(0);
        } else {
            self.tracks = tracks.to_vec();
            self.index = Some(start);
        }
        Some(first)
    }

    /// Turns shuffle on and starts from a random track, like the window's shuffle button.
    pub fn shuffle_play(&mut self, tracks: &[Track]) -> Option<Track> {
        if tracks.is_empty() {
            return None;
        }
        self.shuffle = true;
        let start = (self.rng.next() % tracks.len() as u64) as usize;
        self.play(tracks, start)
    }

    /// Returns the track to load when the queue was empty.
    pub fn enqueue(&mut self, track: Track) -> Option<Track> {
        self.tracks.push(track.clone());
        self.ordered.push(track.clone());
        if self.index.is_some() {
            return None;
        }
        self.index = Some(self.tracks.len() - 1);
        Some(track)
    }

    pub fn jump_to(&mut self, index: usize) -> Option<Track> {
        let track = self.tracks.get(index)?.clone();
        self.index = Some(index);
        Some(track)
    }

    /// `ended` distinguishes a track running out from the user pressing "next".
    pub fn next(&mut self, ended: bool) -> Option<Track> {
        let index = self.index?;
        if ended && self.repeat == Repeat::One {
            return self.current().cloned();
        }
        let next = if index + 1 < self.tracks.len() {
            index + 1
        } else if self.repeat == Repeat::All && !self.tracks.is_empty() {
            0
        } else {
            return None;
        };
        self.jump_to(next)
    }

    /// `None` means the current track should restart instead.
    pub fn previous(&mut self) -> Option<Track> {
        let index = self.index?;
        if index == 0 {
            return None;
        }
        self.jump_to(index - 1)
    }

    /// Indices are positions within `upcoming`.
    pub fn move_upcoming(&mut self, from: usize, to: usize) {
        let Some(index) = self.index else { return };
        let (from, to) = (index + 1 + from, index + 1 + to);
        if from >= self.tracks.len() || to >= self.tracks.len() {
            return;
        }
        let moved = self.tracks.remove(from);
        self.tracks.insert(to, moved);
        self.ordered = self.tracks.clone();
    }

    pub fn shuffle_upcoming(&mut self) {
        let Some(index) = self.index else { return };
        let mut upcoming = self.tracks.split_off(index + 1);
        self.rng.shuffle(&mut upcoming);
        self.tracks.extend(upcoming);
        self.ordered = self.tracks.clone();
    }

    pub fn clear_upcoming(&mut self) {
        let Some(index) = self.index else { return };
        self.tracks.truncate(index + 1);
        self.ordered = self.tracks.clone();
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        let (Some(index), Some(current)) = (self.index, self.current().cloned()) else { return };

        if self.shuffle {
            self.ordered = self.tracks.clone();
            let mut rest = without(&self.tracks, index);
            self.rng.shuffle(&mut rest);
            self.tracks = std::iter::once(current).chain(rest).collect();
            self.index = Some(0);
        } else {
            self.tracks = self.ordered.clone();
            let key = key_of(&current);
            self.index = Some(self.tracks.iter().position(|t| key_of(t) == key).unwrap_or(0));
        }
    }

    pub fn cycle_repeat(&mut self) {
        self.repeat = match self.repeat {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        };
    }
}

fn without(tracks: &[Track], skipped: usize) -> Vec<Track> {
    tracks
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != skipped)
        .map(|(_, track)| track.clone())
        .collect()
}

/// xorshift64*: shuffling a play queue needs no cryptographic randomness or another crate.
struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng(seed | 1)
    }
}

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str) -> Track {
        Track {
            id: id.into(),
            provider: Provider::Youtube,
            title: id.into(),
            artists: Vec::new(),
            album: None,
            duration_ms: None,
            artwork: None,
            url: None,
        }
    }

    fn ids(tracks: &[Track]) -> Vec<&str> {
        tracks.iter().map(|t| t.id.as_str()).collect()
    }

    #[test]
    fn plays_from_the_chosen_track() {
        let mut queue = Queue::default();
        let started = queue.play(&[track("a"), track("b"), track("c")], 1).unwrap();
        assert_eq!(started.id, "b");
        assert_eq!(ids(queue.upcoming()), ["c"]);
    }

    #[test]
    fn next_respects_repeat() {
        let mut queue = Queue::default();
        queue.play(&[track("a"), track("b")], 1);
        assert!(queue.next(true).is_none());

        queue.cycle_repeat();
        assert_eq!(queue.next(false).unwrap().id, "a");

        queue.cycle_repeat();
        assert_eq!(queue.next(true).unwrap().id, "a");
        assert_eq!(queue.next(false).unwrap().id, "b");
    }

    #[test]
    fn shuffle_keeps_current_first_and_restores_order() {
        let mut queue = Queue::default();
        let tracks: Vec<Track> = ["a", "b", "c", "d", "e"].map(track).to_vec();
        queue.play(&tracks, 2);

        queue.toggle_shuffle();
        assert_eq!(queue.current().unwrap().id, "c");
        assert_eq!(queue.index(), Some(0));
        let mut shuffled = ids(queue.tracks());
        shuffled.sort();
        assert_eq!(shuffled, ["a", "b", "c", "d", "e"]);

        queue.toggle_shuffle();
        assert_eq!(ids(queue.tracks()), ["a", "b", "c", "d", "e"]);
        assert_eq!(queue.index(), Some(2));
    }

    #[test]
    fn enqueue_starts_an_empty_queue() {
        let mut queue = Queue::default();
        assert_eq!(queue.enqueue(track("a")).unwrap().id, "a");
        assert!(queue.enqueue(track("b")).is_none());
        assert_eq!(ids(queue.upcoming()), ["b"]);
    }

    #[test]
    fn edits_only_touch_upcoming() {
        let mut queue = Queue::default();
        queue.play(&[track("a"), track("b"), track("c"), track("d")], 1);
        queue.move_upcoming(1, 0);
        assert_eq!(ids(queue.tracks()), ["a", "b", "d", "c"]);
        queue.clear_upcoming();
        assert_eq!(ids(queue.tracks()), ["a", "b"]);
        assert!(queue.previous().is_some());
        assert!(queue.previous().is_none());
    }
}
