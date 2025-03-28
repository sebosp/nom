//! Error management
//!
//! Parsers are generic over their error type, requiring that it implements
//! the `error::ParseError<Input>` trait.

use crate::internal::{Err, Mode, OutputMode, PResult, Parser};
use crate::lib::std::fmt;

#[cfg(feature = "alloc")]
use crate::alloc::borrow::ToOwned;

#[cfg(feature = "std")]
use crate::internal::IResult;

/// This trait must be implemented by the error type of a nom parser.
///
/// There are already implementations of it for `(Input, ErrorKind)`
/// and `Error<Input>`.
///
/// It provides methods to create an error from some combinators,
/// and combine existing errors in combinators like `alt`.
pub trait ParseError<I>: Sized {
  /// Creates an error from the input position and an [ErrorKind]
  fn from_error_kind(input: I, kind: ErrorKind) -> Self;

  /// Combines an existing error with a new one created from the input
  /// position and an [ErrorKind]. This is useful when backtracking
  /// through a parse tree, accumulating error context on the way
  fn append(input: I, kind: ErrorKind, other: Self) -> Self;

  /// Creates an error from an input position and an expected character
  fn from_char(input: I, _: char) -> Self {
    Self::from_error_kind(input, ErrorKind::Char)
  }

  /// Combines two existing errors. This function is used to compare errors
  /// generated in various branches of `alt`.
  fn or(self, other: Self) -> Self {
    other
  }
}

/// This trait is required by the `context` combinator to add a static string
/// to an existing error
pub trait ContextError<I>: Sized {
  /// Creates a new error from an input position, a static string and an existing error.
  /// This is used mainly in the [context] combinator, to add user friendly information
  /// to errors when backtracking through a parse tree
  fn add_context(_input: I, _ctx: &'static str, other: Self) -> Self {
    other
  }
}

/// This trait is required by the `map_res` combinator to integrate
/// error types from external functions, like [std::str::FromStr]
pub trait FromExternalError<I, E> {
  /// Creates a new error from an input position, an [ErrorKind] indicating the
  /// wrapping parser, and an external error
  fn from_external_error(input: I, kind: ErrorKind, e: E) -> Self;
}

/// default error type, only contains the error's location and code
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error<I> {
  /// position of the error in the input data
  pub input: I,
  /// nom error code
  pub code: ErrorKind,
}

impl<I> Error<I> {
  /// creates a new basic error
  pub fn new(input: I, code: ErrorKind) -> Error<I> {
    Error { input, code }
  }
}

impl<I> ParseError<I> for Error<I> {
  fn from_error_kind(input: I, kind: ErrorKind) -> Self {
    Error { input, code: kind }
  }

  fn append(_: I, _: ErrorKind, other: Self) -> Self {
    other
  }
}

impl<I> ContextError<I> for Error<I> {}

impl<I, E> FromExternalError<I, E> for Error<I> {
  /// Create a new error from an input position and an external error
  fn from_external_error(input: I, kind: ErrorKind, _e: E) -> Self {
    Error { input, code: kind }
  }
}

/// The Display implementation allows the std::error::Error implementation
impl<I: fmt::Display> fmt::Display for Error<I> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "error {:?} at: {}", self.code, self.input)
  }
}

#[cfg(feature = "alloc")]
impl<I: ToOwned + ?Sized> Error<&I> {
  /// Converts `Error<&I>` into `Error<I::Owned>` by cloning.
  pub fn cloned(self) -> Error<I::Owned> {
    Error {
      input: self.input.to_owned(),
      code: self.code,
    }
  }
}

#[cfg(feature = "alloc")]
impl<I: ToOwned + ?Sized> Error<&mut I> {
  /// Converts `Error<&mut I>` into `Error<I::Owned>` by cloning.
  pub fn cloned(self) -> Error<I::Owned> {
    Error {
      input: self.input.to_owned(),
      code: self.code,
    }
  }
}

impl<I: Copy> Error<&I> {
  /// Converts `Error<&I>` into `Error<I>` by copying.
  pub fn copied(self) -> Error<I> {
    Error {
      input: *self.input,
      code: self.code,
    }
  }
}

impl<I: Copy> Error<&mut I> {
  /// Converts `Error<&mut I>` into `Error<I>` by copying.
  pub fn copied(self) -> Error<I> {
    Error {
      input: *self.input,
      code: self.code,
    }
  }
}

#[cfg(feature = "std")]
impl<I: fmt::Debug + fmt::Display> std::error::Error for Error<I> {}

#[cfg(feature = "alloc")]
#[cfg_attr(feature = "docsrs", doc(cfg(feature = "alloc")))]
impl From<Error<&[u8]>> for Error<crate::lib::std::vec::Vec<u8>> {
  fn from(value: Error<&[u8]>) -> Self {
    Error {
      input: value.input.to_owned(),
      code: value.code,
    }
  }
}

#[cfg(feature = "alloc")]
#[cfg_attr(feature = "docsrs", doc(cfg(feature = "alloc")))]
impl From<Error<&str>> for Error<crate::lib::std::string::String> {
  fn from(value: Error<&str>) -> Self {
    Error {
      input: value.input.to_owned(),
      code: value.code,
    }
  }
}

// for backward compatibility, keep those trait implementations
// for the previously used error type
impl<I> ParseError<I> for (I, ErrorKind) {
  fn from_error_kind(input: I, kind: ErrorKind) -> Self {
    (input, kind)
  }

  fn append(_: I, _: ErrorKind, other: Self) -> Self {
    other
  }
}

impl<I> ContextError<I> for (I, ErrorKind) {}

impl<I, E> FromExternalError<I, E> for (I, ErrorKind) {
  fn from_external_error(input: I, kind: ErrorKind, _e: E) -> Self {
    (input, kind)
  }
}

impl<I> ParseError<I> for () {
  fn from_error_kind(_: I, _: ErrorKind) -> Self {}

  fn append(_: I, _: ErrorKind, _: Self) -> Self {}
}

impl<I> ContextError<I> for () {}

impl<I, E> FromExternalError<I, E> for () {
  fn from_external_error(_input: I, _kind: ErrorKind, _e: E) -> Self {}
}

/// Creates an error from the input position and an [ErrorKind]
pub fn make_error<I, E: ParseError<I>>(input: I, kind: ErrorKind) -> E {
  E::from_error_kind(input, kind)
}

/// Combines an existing error with a new one created from the input
/// position and an [ErrorKind]. This is useful when backtracking
/// through a parse tree, accumulating error context on the way
pub fn append_error<I, E: ParseError<I>>(input: I, kind: ErrorKind, other: E) -> E {
  E::append(input, kind, other)
}

/// Create a new error from an input position, a static string and an existing error.
/// This is used mainly in the [context] combinator, to add user friendly information
/// to errors when backtracking through a parse tree
pub fn context<F>(context: &'static str, parser: F) -> Context<F> {
  Context { context, parser }
}

/// Parser implementation for [context]
pub struct Context<F> {
  context: &'static str,
  parser: F,
}

impl<I, F> Parser<I> for Context<F>
where
  I: Clone,
  F: Parser<I>,
  <F as Parser<I>>::Error: ContextError<I>,
{
  type Output = <F as Parser<I>>::Output;
  type Error = <F as Parser<I>>::Error;

  fn process<OM: OutputMode>(&mut self, input: I) -> PResult<OM, I, Self::Output, Self::Error> {
    match self.parser.process::<OM>(input.clone()) {
      Err(Err::Error(e)) => Err(Err::Error(OM::Error::map(e, |e| {
        <F as Parser<I>>::Error::add_context(input, self.context, e)
      }))),
      Err(Err::Failure(e)) => Err(Err::Failure(<F as Parser<I>>::Error::add_context(
        input,
        self.context,
        e,
      ))),
      x => x,
    }
  }
}

/// Indicates which parser returned an error
#[rustfmt::skip]
#[derive(Debug,PartialEq,Eq,Hash,Clone,Copy)]
#[allow(deprecated,missing_docs)]
pub enum ErrorKind {
  Tag,
  MapRes,
  MapOpt,
  Alt,
  IsNot,
  IsA,
  SeparatedList,
  SeparatedNonEmptyList,
  Many0,
  Many1,
  ManyTill,
  Count,
  TakeUntil,
  LengthValue,
  TagClosure,
  Alpha,
  Digit,
  HexDigit,
  OctDigit,
  BinDigit,
  AlphaNumeric,
  Space,
  MultiSpace,
  LengthValueFn,
  Eof,
  Switch,
  TagBits,
  OneOf,
  NoneOf,
  Char,
  CrLf,
  RegexpMatch,
  RegexpMatches,
  RegexpFind,
  RegexpCapture,
  RegexpCaptures,
  TakeWhile1,
  Complete,
  Fix,
  Escaped,
  EscapedTransform,
  NonEmpty,
  ManyMN,
  Not,
  Permutation,
  Verify,
  TakeTill1,
  TakeWhileMN,
  TooLarge,
  Many0Count,
  Many1Count,
  Float,
  Satisfy,
  Fail,
  Many,
  Fold,
  Precedence,
}

#[rustfmt::skip]
#[allow(deprecated)]
/// Converts an ErrorKind to a number
pub fn error_to_u32(e: &ErrorKind) -> u32 {
  match *e {
    ErrorKind::Tag                       => 1,
    ErrorKind::MapRes                    => 2,
    ErrorKind::MapOpt                    => 3,
    ErrorKind::Alt                       => 4,
    ErrorKind::IsNot                     => 5,
    ErrorKind::IsA                       => 6,
    ErrorKind::SeparatedList             => 7,
    ErrorKind::SeparatedNonEmptyList     => 8,
    ErrorKind::Many1                     => 9,
    ErrorKind::Count                     => 10,
    ErrorKind::TakeUntil                 => 12,
    ErrorKind::LengthValue               => 15,
    ErrorKind::TagClosure                => 16,
    ErrorKind::Alpha                     => 17,
    ErrorKind::Digit                     => 18,
    ErrorKind::AlphaNumeric              => 19,
    ErrorKind::Space                     => 20,
    ErrorKind::MultiSpace                => 21,
    ErrorKind::LengthValueFn             => 22,
    ErrorKind::Eof                       => 23,
    ErrorKind::Switch                    => 27,
    ErrorKind::TagBits                   => 28,
    ErrorKind::OneOf                     => 29,
    ErrorKind::NoneOf                    => 30,
    ErrorKind::Char                      => 40,
    ErrorKind::CrLf                      => 41,
    ErrorKind::RegexpMatch               => 42,
    ErrorKind::RegexpMatches             => 43,
    ErrorKind::RegexpFind                => 44,
    ErrorKind::RegexpCapture             => 45,
    ErrorKind::RegexpCaptures            => 46,
    ErrorKind::TakeWhile1                => 47,
    ErrorKind::Complete                  => 48,
    ErrorKind::Fix                       => 49,
    ErrorKind::Escaped                   => 50,
    ErrorKind::EscapedTransform          => 51,
    ErrorKind::NonEmpty                  => 56,
    ErrorKind::ManyMN                    => 57,
    ErrorKind::HexDigit                  => 59,
    ErrorKind::OctDigit                  => 61,
    ErrorKind::Many0                     => 62,
    ErrorKind::Not                       => 63,
    ErrorKind::Permutation               => 64,
    ErrorKind::ManyTill                  => 65,
    ErrorKind::Verify                    => 66,
    ErrorKind::TakeTill1                 => 67,
    ErrorKind::TakeWhileMN               => 69,
    ErrorKind::TooLarge                  => 70,
    ErrorKind::Many0Count                => 71,
    ErrorKind::Many1Count                => 72,
    ErrorKind::Float                     => 73,
    ErrorKind::Satisfy                   => 74,
    ErrorKind::Fail                      => 75,
    ErrorKind::Many                      => 76,
    ErrorKind::Fold                      => 77,
    ErrorKind::BinDigit                  => 78,
    ErrorKind::Precedence                => 79,
  }
}

impl ErrorKind {
  #[rustfmt::skip]
  #[allow(deprecated)]
  /// Converts an ErrorKind to a text description
  pub fn description(&self) -> &str {
    match *self {
      ErrorKind::Tag                       => "Tag",
      ErrorKind::MapRes                    => "Map on Result",
      ErrorKind::MapOpt                    => "Map on Option",
      ErrorKind::Alt                       => "Alternative",
      ErrorKind::IsNot                     => "IsNot",
      ErrorKind::IsA                       => "IsA",
      ErrorKind::SeparatedList             => "Separated list",
      ErrorKind::SeparatedNonEmptyList     => "Separated non empty list",
      ErrorKind::Many0                     => "Many0",
      ErrorKind::Many1                     => "Many1",
      ErrorKind::Count                     => "Count",
      ErrorKind::TakeUntil                 => "Take until",
      ErrorKind::LengthValue               => "Length followed by value",
      ErrorKind::TagClosure                => "Tag closure",
      ErrorKind::Alpha                     => "Alphabetic",
      ErrorKind::Digit                     => "Digit",
      ErrorKind::AlphaNumeric              => "AlphaNumeric",
      ErrorKind::Space                     => "Space",
      ErrorKind::MultiSpace                => "Multiple spaces",
      ErrorKind::LengthValueFn             => "LengthValueFn",
      ErrorKind::Eof                       => "End of file",
      ErrorKind::Switch                    => "Switch",
      ErrorKind::TagBits                   => "Tag on bitstream",
      ErrorKind::OneOf                     => "OneOf",
      ErrorKind::NoneOf                    => "NoneOf",
      ErrorKind::Char                      => "Char",
      ErrorKind::CrLf                      => "CrLf",
      ErrorKind::RegexpMatch               => "RegexpMatch",
      ErrorKind::RegexpMatches             => "RegexpMatches",
      ErrorKind::RegexpFind                => "RegexpFind",
      ErrorKind::RegexpCapture             => "RegexpCapture",
      ErrorKind::RegexpCaptures            => "RegexpCaptures",
      ErrorKind::TakeWhile1                => "TakeWhile1",
      ErrorKind::Complete                  => "Complete",
      ErrorKind::Fix                       => "Fix",
      ErrorKind::Escaped                   => "Escaped",
      ErrorKind::EscapedTransform          => "EscapedTransform",
      ErrorKind::NonEmpty                  => "NonEmpty",
      ErrorKind::ManyMN                    => "Many(m, n)",
      ErrorKind::HexDigit                  => "Hexadecimal Digit",
      ErrorKind::OctDigit                  => "Octal digit",
      ErrorKind::BinDigit                  => "Binary digit",
      ErrorKind::Not                       => "Negation",
      ErrorKind::Permutation               => "Permutation",
      ErrorKind::ManyTill                  => "ManyTill",
      ErrorKind::Verify                    => "predicate verification",
      ErrorKind::TakeTill1                 => "TakeTill1",
      ErrorKind::TakeWhileMN               => "TakeWhileMN",
      ErrorKind::TooLarge                  => "Needed data size is too large",
      ErrorKind::Many0Count                => "Count occurrence of >=0 patterns",
      ErrorKind::Many1Count                => "Count occurrence of >=1 patterns",
      ErrorKind::Float                     => "Float",
      ErrorKind::Satisfy                   => "Satisfy",
      ErrorKind::Fail                      => "Fail",
      ErrorKind::Many                      => "Many",
      ErrorKind::Fold                      => "Fold",
      ErrorKind::Precedence                => "Precedence",
    }
  }
}

/// Creates a parse error from a `nom::ErrorKind`
/// and the position in the input
#[allow(unused_variables)]
#[macro_export(local_inner_macros)]
macro_rules! error_position(
  ($input:expr, $code:expr $(,)?) => ({
    $crate::error::make_error($input, $code)
  });
);

/// Creates a parse error from a `nom::ErrorKind`,
/// the position in the input and the next error in
/// the parsing tree
#[allow(unused_variables)]
#[macro_export(local_inner_macros)]
macro_rules! error_node_position(
  ($input:expr, $code:expr, $next:expr $(,)?) => ({
    $crate::error::append_error($input, $code, $next)
  });
);

/// Prints a message and the input if the parser fails.
///
/// The message prints the `Error` or `Incomplete`
/// and the parser's calling code.
///
/// It also displays the input in hexdump format
///
/// ```rust
/// use nom::{IResult, error::dbg_dmp, bytes::complete::tag};
///
/// fn f(i: &[u8]) -> IResult<&[u8], &[u8]> {
///   dbg_dmp(tag("abcd"), "tag")(i)
/// }
///
///   let a = &b"efghijkl"[..];
///
/// // Will print the following message:
/// // Error(Position(0, [101, 102, 103, 104, 105, 106, 107, 108])) at l.5 by ' tag ! ( "abcd" ) '
/// // 00000000        65 66 67 68 69 6a 6b 6c         efghijkl
/// f(a);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(feature = "docsrs", doc(cfg(feature = "std")))]
pub fn dbg_dmp<'a, F, O, E: std::fmt::Debug>(
  mut f: F,
  context: &'static str,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], O, E>
where
  F: FnMut(&'a [u8]) -> IResult<&'a [u8], O, E>,
{
  use crate::HexDisplay;
  move |i: &'a [u8]| match f(i) {
    Err(e) => {
      println!("{}: Error({:?}) at:\n{}", context, e, i.to_hex(8));
      Err(e)
    }
    a => a,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn context_test() {
    use crate::{character::char, combinator::cut, internal::Needed};

    #[derive(Debug, PartialEq)]
    struct Error<I> {
      input: I,
      ctx: Option<&'static str>,
    }

    impl<I> ParseError<I> for Error<I> {
      fn from_error_kind(input: I, _kind: ErrorKind) -> Self {
        Self { input, ctx: None }
      }

      fn append(input: I, _kind: ErrorKind, other: Self) -> Self {
        Self {
          input,
          ctx: other.ctx,
        }
      }
    }

    impl<I> ContextError<I> for Error<I> {
      fn add_context(input: I, ctx: &'static str, _other: Self) -> Self {
        Self {
          input,
          ctx: Some(ctx),
        }
      }
    }

    assert_eq!(
      context("ctx", char::<_, Error<_>>('a')).parse("abcd"),
      Ok(("bcd", 'a'))
    );
    assert_eq!(
      context("ctx", char::<_, Error<_>>('a')).parse(""),
      Err(Err::Incomplete(Needed::new(1)))
    );
    assert_eq!(
      context("ctx", char::<_, Error<_>>('a')).parse_complete(""),
      Err(Err::Error(Error {
        input: "",
        ctx: Some("ctx")
      }))
    );
    assert_eq!(
      context("ctx", cut(char::<_, Error<_>>('a'))).parse("bcd"),
      Err(Err::Failure(Error {
        input: "bcd",
        ctx: Some("ctx")
      }))
    );
  }

  #[cfg(feature = "alloc")]
  #[test]
  fn clone_error() {
    use crate::lib::std::string::String;
    let err = Error {
      code: ErrorKind::Eof,
      input: "test",
    };

    let _err: Error<String> = err.cloned();
  }

  #[test]
  fn copy_error() {
    let err = Error {
      code: ErrorKind::Eof,
      input: &0_u8,
    };

    let _err: Error<u8> = err.copied();
  }
}

/*
#[cfg(feature = "alloc")]
use lib::std::{vec::Vec, collections::HashMap};

#[cfg(feature = "std")]
use lib::std::hash::Hash;

#[cfg(feature = "std")]
pub fn add_error_pattern<'a, I: Clone + Hash + Eq, O, E: Clone + Hash + Eq>(
  h: &mut HashMap<VerboseError<I>, &'a str>,
  e: VerboseError<I>,
  message: &'a str,
) -> bool {
  h.insert(e, message);
  true
}

pub fn slice_to_offsets(input: &[u8], s: &[u8]) -> (usize, usize) {
  let start = input.as_ptr();
  let off1 = s.as_ptr() as usize - start as usize;
  let off2 = off1 + s.len();
  (off1, off2)
}

#[cfg(feature = "std")]
pub fn prepare_errors<O, E: Clone>(input: &[u8], e: VerboseError<&[u8]>) -> Option<Vec<(ErrorKind, usize, usize)>> {
  let mut v: Vec<(ErrorKind, usize, usize)> = Vec::new();

  for (p, kind) in e.errors.drain(..) {
    let (o1, o2) = slice_to_offsets(input, p);
    v.push((kind, o1, o2));
  }

  v.reverse();
  Some(v)
}

#[cfg(feature = "std")]
pub fn print_error<O, E: Clone>(input: &[u8], res: VerboseError<&[u8]>) {
  if let Some(v) = prepare_errors(input, res) {
    let colors = generate_colors(&v);
    println!("parser codes: {}", print_codes(&colors, &HashMap::new()));
    println!("{}", print_offsets(input, 0, &v));
  } else {
    println!("not an error");
  }
}

#[cfg(feature = "std")]
pub fn generate_colors<E>(v: &[(ErrorKind, usize, usize)]) -> HashMap<u32, u8> {
  let mut h: HashMap<u32, u8> = HashMap::new();
  let mut color = 0;

  for &(ref c, _, _) in v.iter() {
    h.insert(error_to_u32(c), color + 31);
    color = color + 1 % 7;
  }

  h
}

pub fn code_from_offset(v: &[(ErrorKind, usize, usize)], offset: usize) -> Option<u32> {
  let mut acc: Option<(u32, usize, usize)> = None;
  for &(ref ek, s, e) in v.iter() {
    let c = error_to_u32(ek);
    if s <= offset && offset <= e {
      if let Some((_, start, end)) = acc {
        if start <= s && e <= end {
          acc = Some((c, s, e));
        }
      } else {
        acc = Some((c, s, e));
      }
    }
  }
  if let Some((code, _, _)) = acc {
    return Some(code);
  } else {
    return None;
  }
}

#[cfg(feature = "alloc")]
pub fn reset_color(v: &mut Vec<u8>) {
  v.push(0x1B);
  v.push(b'[');
  v.push(0);
  v.push(b'm');
}

#[cfg(feature = "alloc")]
pub fn write_color(v: &mut Vec<u8>, color: u8) {
  v.push(0x1B);
  v.push(b'[');
  v.push(1);
  v.push(b';');
  let s = color.to_string();
  let bytes = s.as_bytes();
  v.extend(bytes.iter().cloned());
  v.push(b'm');
}

#[cfg(feature = "std")]
#[cfg_attr(feature = "cargo-clippy", allow(implicit_hasher))]
pub fn print_codes(colors: &HashMap<u32, u8>, names: &HashMap<u32, &str>) -> String {
  let mut v = Vec::new();
  for (code, &color) in colors {
    if let Some(&s) = names.get(code) {
      let bytes = s.as_bytes();
      write_color(&mut v, color);
      v.extend(bytes.iter().cloned());
    } else {
      let s = code.to_string();
      let bytes = s.as_bytes();
      write_color(&mut v, color);
      v.extend(bytes.iter().cloned());
    }
    reset_color(&mut v);
    v.push(b' ');
  }
  reset_color(&mut v);

  String::from_utf8_lossy(&v[..]).into_owned()
}

#[cfg(feature = "std")]
pub fn print_offsets(input: &[u8], from: usize, offsets: &[(ErrorKind, usize, usize)]) -> String {
  let mut v = Vec::with_capacity(input.len() * 3);
  let mut i = from;
  let chunk_size = 8;
  let mut current_code: Option<u32> = None;
  let mut current_code2: Option<u32> = None;

  let colors = generate_colors(&offsets);

  for chunk in input.chunks(chunk_size) {
    let s = format!("{:08x}", i);
    for &ch in s.as_bytes().iter() {
      v.push(ch);
    }
    v.push(b'\t');

    let mut k = i;
    let mut l = i;
    for &byte in chunk {
      if let Some(code) = code_from_offset(&offsets, k) {
        if let Some(current) = current_code {
          if current != code {
            reset_color(&mut v);
            current_code = Some(code);
            if let Some(&color) = colors.get(&code) {
              write_color(&mut v, color);
            }
          }
        } else {
          current_code = Some(code);
          if let Some(&color) = colors.get(&code) {
            write_color(&mut v, color);
          }
        }
      }
      v.push(CHARS[(byte >> 4) as usize]);
      v.push(CHARS[(byte & 0xf) as usize]);
      v.push(b' ');
      k = k + 1;
    }

    reset_color(&mut v);

    if chunk_size > chunk.len() {
      for _ in 0..(chunk_size - chunk.len()) {
        v.push(b' ');
        v.push(b' ');
        v.push(b' ');
      }
    }
    v.push(b'\t');

    for &byte in chunk {
      if let Some(code) = code_from_offset(&offsets, l) {
        if let Some(current) = current_code2 {
          if current != code {
            reset_color(&mut v);
            current_code2 = Some(code);
            if let Some(&color) = colors.get(&code) {
              write_color(&mut v, color);
            }
          }
        } else {
          current_code2 = Some(code);
          if let Some(&color) = colors.get(&code) {
            write_color(&mut v, color);
          }
        }
      }
      if (byte >= 32 && byte <= 126) || byte >= 128 {
        v.push(byte);
      } else {
        v.push(b'.');
      }
      l = l + 1;
    }
    reset_color(&mut v);

    v.push(b'\n');
    i = i + chunk_size;
  }

  String::from_utf8_lossy(&v[..]).into_owned()
}
*/
