//! Interface v2: input data (bars, settings) and outputs (lines, labels,
//! cards), in the fixed little-endian layout of `docs/abi-v2.md`. Prices and
//! values are integers at a fixed scale (decimals): no floating point in the
//! interface. Decoding never panics and refuses anything out of bounds.

use alloc::string::String;
use alloc::vec::Vec;

/// Version of this interface.
pub const ABI: u32 = 2;
/// "No value" in a line (integers have no NaN).
pub const EMPTY: i64 = i64::MIN;
/// Largest encoded input or output, in bytes (4 MiB).
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
/// Most bars in one input.
pub const MAX_BARS: usize = 50_000;
/// Most settings.
pub const MAX_SETTINGS: usize = 16;
/// Most lines in one output.
pub const MAX_LINES: usize = 16;
/// Most labels in one output.
pub const MAX_LABELS: usize = 1000;
/// Most cards in one output.
pub const MAX_CARDS: usize = 32;
/// Largest scale (decimals).
pub const MAX_SCALE: u8 = 12;

/// One bar: time (Unix seconds) and open, high, low, close, volume at the
/// input's scales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bar {
    /// Unix seconds.
    pub t: i64,
    /// Open, at `price_scale`.
    pub o: i64,
    /// High, at `price_scale`.
    pub h: i64,
    /// Low, at `price_scale`.
    pub l: i64,
    /// Close, at `price_scale`.
    pub c: i64,
    /// Volume, at `volume_scale`.
    pub v: i64,
}

/// What the host gives the module.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Input {
    /// Bar duration in seconds.
    pub timeframe: u32,
    /// Decimals of prices (`12345` at scale 2 is 123.45).
    pub price_scale: u8,
    /// Decimals of volumes.
    pub volume_scale: u8,
    /// Market symbol.
    pub symbol: String,
    /// Bars, oldest first, times strictly increasing.
    pub bars: Vec<Bar>,
    /// Setting values, in the order of the manifest.
    pub settings: Vec<i64>,
}

/// One curve: one value per bar ([`EMPTY`] = none).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Line {
    /// Name shown in the chart.
    pub name: String,
    /// Decimals of the values.
    pub scale: u8,
    /// One value per input bar.
    pub values: Vec<i64>,
}

/// A text placed on one bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Label {
    /// Index of the bar.
    pub bar: u32,
    /// Text.
    pub text: String,
}

/// A "name · value" line for a dashboard card.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Card {
    /// Name.
    pub name: String,
    /// Value at `scale`.
    pub value: i64,
    /// Decimals of the value.
    pub scale: u8,
}

/// What the module gives back.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Output {
    /// Curves.
    pub lines: Vec<Line>,
    /// Texts on bars.
    pub labels: Vec<Label>,
    /// Dashboard lines.
    pub cards: Vec<Card>,
}

/// Why bytes are refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Shorter than announced.
    Truncated,
    /// Bytes left after the last field.
    Trailing,
    /// Not version 2.
    Version,
    /// A count, length or scale out of bounds, or bad UTF-8.
    Invalid,
    /// More than [`MAX_BYTES`].
    TooLarge,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], DecodeError> {
        let end = self.at.checked_add(n).ok_or(DecodeError::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(DecodeError::Truncated)?;
        self.at = end;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, DecodeError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i64(&mut self) -> Result<i64, DecodeError> {
        let b = self.take(8)?;
        Ok(i64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn scale(&mut self) -> Result<u8, DecodeError> {
        let s = self.u8()?;
        if s > MAX_SCALE {
            return Err(DecodeError::Invalid);
        }
        Ok(s)
    }
    fn string(&mut self, min: usize, max: usize) -> Result<String, DecodeError> {
        let n = usize::from(self.u16()?);
        if n < min || n > max {
            return Err(DecodeError::Invalid);
        }
        let b = self.take(n)?;
        core::str::from_utf8(b)
            .map(String::from)
            .map_err(|_| DecodeError::Invalid)
    }
    fn count(&mut self, n: usize, max: usize, item_bytes: usize) -> Result<usize, DecodeError> {
        if n > max {
            return Err(DecodeError::Invalid);
        }
        // Never reserve more than the bytes left can hold.
        let left = self.bytes.len().saturating_sub(self.at);
        if n.saturating_mul(item_bytes) > left {
            return Err(DecodeError::Truncated);
        }
        Ok(n)
    }
    fn end(&self) -> Result<(), DecodeError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(DecodeError::Trailing)
        }
    }
}

fn start(bytes: &[u8]) -> Result<Reader<'_>, DecodeError> {
    if bytes.len() > MAX_BYTES {
        return Err(DecodeError::TooLarge);
    }
    let mut r = Reader { bytes, at: 0 };
    if r.u32()? != ABI {
        return Err(DecodeError::Version);
    }
    Ok(r)
}

#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i64(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn len(&mut self, n: usize) {
        self.u16(u16::try_from(n).unwrap_or(u16::MAX));
    }
    fn string(&mut self, s: &str) {
        self.len(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
}

impl Input {
    /// Encodes the input. Check the result with [`Input::decode`] to know it
    /// is within limits.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.u32(ABI);
        w.u32(self.timeframe);
        w.u8(self.price_scale);
        w.u8(self.volume_scale);
        w.string(&self.symbol);
        w.u32(u32::try_from(self.bars.len()).unwrap_or(u32::MAX));
        for b in &self.bars {
            for v in [b.t, b.o, b.h, b.l, b.c, b.v] {
                w.i64(v);
            }
        }
        w.len(self.settings.len());
        for s in &self.settings {
            w.i64(*s);
        }
        w.0
    }

    /// Decodes and checks an input.
    ///
    /// # Errors
    /// [`DecodeError`] for anything out of the layout or its limits.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = start(bytes)?;
        let timeframe = r.u32()?;
        let price_scale = r.scale()?;
        let volume_scale = r.scale()?;
        let symbol = r.string(1, 32)?;
        if timeframe == 0 {
            return Err(DecodeError::Invalid);
        }
        let n = usize::try_from(r.u32()?).map_err(|_| DecodeError::Invalid)?;
        let n = r.count(n, MAX_BARS, 48)?;
        let mut bars = Vec::with_capacity(n);
        for _ in 0..n {
            let bar = Bar {
                t: r.i64()?,
                o: r.i64()?,
                h: r.i64()?,
                l: r.i64()?,
                c: r.i64()?,
                v: r.i64()?,
            };
            if bars.last().is_some_and(|p: &Bar| p.t >= bar.t) {
                return Err(DecodeError::Invalid);
            }
            bars.push(bar);
        }
        let s = usize::from(r.u16()?);
        let s = r.count(s, MAX_SETTINGS, 8)?;
        let mut settings = Vec::with_capacity(s);
        for _ in 0..s {
            settings.push(r.i64()?);
        }
        r.end()?;
        Ok(Self {
            timeframe,
            price_scale,
            volume_scale,
            symbol,
            bars,
            settings,
        })
    }
}

impl Output {
    /// Encodes the output.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.u32(ABI);
        w.len(self.lines.len());
        for line in &self.lines {
            w.string(&line.name);
            w.u8(line.scale);
            w.u32(u32::try_from(line.values.len()).unwrap_or(u32::MAX));
            for v in &line.values {
                w.i64(*v);
            }
        }
        w.len(self.labels.len());
        for label in &self.labels {
            w.u32(label.bar);
            w.string(&label.text);
        }
        w.len(self.cards.len());
        for card in &self.cards {
            w.string(&card.name);
            w.i64(card.value);
            w.u8(card.scale);
        }
        w.0
    }

    /// Decodes and checks an output for an input of `bars` bars.
    ///
    /// # Errors
    /// [`DecodeError`] for anything out of the layout or its limits.
    pub fn decode(bytes: &[u8], bars: usize) -> Result<Self, DecodeError> {
        let mut r = start(bytes)?;
        let n = usize::from(r.u16()?);
        let n = r.count(n, MAX_LINES, 7)?;
        let mut lines = Vec::with_capacity(n);
        for _ in 0..n {
            let name = r.string(1, 64)?;
            let scale = r.scale()?;
            let count = usize::try_from(r.u32()?).map_err(|_| DecodeError::Invalid)?;
            if count != bars {
                return Err(DecodeError::Invalid);
            }
            let count = r.count(count, MAX_BARS, 8)?;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push(r.i64()?);
            }
            lines.push(Line {
                name,
                scale,
                values,
            });
        }
        let n = usize::from(r.u16()?);
        let n = r.count(n, MAX_LABELS, 7)?;
        let mut labels = Vec::with_capacity(n);
        for _ in 0..n {
            let bar = r.u32()?;
            if usize::try_from(bar).map_or(true, |b| b >= bars) {
                return Err(DecodeError::Invalid);
            }
            labels.push(Label {
                bar,
                text: r.string(1, 256)?,
            });
        }
        let n = usize::from(r.u16()?);
        let n = r.count(n, MAX_CARDS, 13)?;
        let mut cards = Vec::with_capacity(n);
        for _ in 0..n {
            cards.push(Card {
                name: r.string(1, 64)?,
                value: r.i64()?,
                scale: r.scale()?,
            });
        }
        r.end()?;
        Ok(Self {
            lines,
            labels,
            cards,
        })
    }
}
