/*!
Additional documentation.

# Components

These tables summarise the provided string components.  Prefixes are used in type aliases and debug output.

## Structures

See the `structure` module.

| Prefix | Name          | Structure |
| ------ | ------------- | --------- |
| `Bstr` | `Bstr`        | Pointer to sequence of units, with 32-bit length in *bytes* stored prior to the first unit.  Has two terminating zero *bytes*.  Requires the `WinSysAlloc` allocator. |
| `Go`   | `Go`          | (pointer, length) pair, where the length is signed.  *Not* zero-terminated. |
| `P`    | `Prefix`      | Pointer to sequence of units, with pointer-sized length in units stored prior to the first unit.  Zero-terminated. |
| `S`    | `Slice`       | (pointer, length) pair.  *Not* zero-terminated. |
| `Z`    | `ZeroTerm`    | Pointer to sequence of units, terminated by a zero (*a.k.a.* null) unit. |
| `Zz`   | `DblZeroTerm` | Pointer to sequence of units, terminated by two zero (*a.k.a.* null) units. |

## Encodings

See the `encoding` module.

| Prefix  | Name        | Encoding |
| ------- | ----------- | -------- |
| `A`     | `Ascii`     | 7-bit ASCII. |
| `Jni`   | `JniMtf8`   | JNI-style "modified" UTF-8. |
| `L`     | `Latin1`    | 8-bit Latin-1. |
| `Mb`    | `MultiByte` | Current thread-local C runtime multibyte encoding. |
| `Raw8`  | `Raw8`      | Raw 8-bit data. |
| `Raw16` | `Raw16`     | Raw 16-bit data. |
| `Utf8`  | `Utf8`      | Possibly invalid UTF-8. |
| `Utf16` | `Utf16`     | Possibly invalid UTF-16. |
| `Utf32` | `Utf32`     | Possibly invalid UTF-32. |
| `U`     | `CheckedUnicode` | Guaranteed valid Unicode.  Should **not** be used for FFI. |
| `W`     | `Wide`      | Current thread-local C runtime wide character encoding. |
| `Wa`    | `WinAnsi`   | Current thread-local Windows ANSI code page. |
| `Ww`    | `WinUnicode`| Equivalent to `Utf16`, assuming non-pathological compiler settings. |

## Allocators

See the `alloc` module.

| Prefix | Name         | Allocator |
| ------ | ------------ | --------- |
| `C`    | `Malloc`     | C runtime heap allocator (*i.e.* `malloc`/`free`) |
| `R`    | `Rust`       | Rust heap allocator. |
| `Wsa`  | `WinSysAlloc` | Windows API `SysAlloc*` allocator.  Requires the `Bstr` structure. |

# Common Misconceptions and Mistakes

* *"Code that deals with text makes some kind of sense."*  It doesn't.  *Lasciate ogne speranza, voi ch'intrate.*

* *"Rust's `CStr` and `CString` are for C strings."*  They're for UTF-8 encoded, zero-terminated strings, with `CString` allocated by Rust, not C.

* *"The C multibyte encoding is UTF-8."*  It merely *defaults* to UTF-8 on some systems.

* *"The C multibyte encoding can be set to UTF-8."*  Not on Windows.  Although Windows defines a UTF-8 codepage for explicit conversions, one cannot actually set it as the active codepage.

* *"The C multibyte encoding and Windows ANSI are the same thing."*  They can be set independently to different code pages.

* *"The C multibyte encoding is at least compatible with ASCII."*  Even if you discount the vanishingly rare EBCDIC, there are multibyte encodings that use multiple bytes *per unit*, and other encodings can shift between single- and multibyte units on the fly.  None of these are ASCII-compatible.

* *"The C wide encoding is UTF-32."*  On Windows, it's UTF-16.

* *"The C wide encoding is some form of Unicode."*  That's not guaranteed *anywhere.*

* *"The C wide encoding is a superset of all possible C multibyte encodings."*  That's not guaranteed, *either.*  Even Unicode does not represent every possible character without loss of information.

* *"There is only ever one C wide encoding."*  GCC allows it to be reconfigured at compilation.

* *"C11's new `char16_t` and `char32_t` are UTF-16 and UTF-32 respectively."*  They are specifically *not* required to be UTF-16 or UTF-32.  They don't even have to be Unicode.  Yes, they apparently thought this was a good idea.  No, I *cannot* imagine *why*.  C++ *does* define them to be UTF-*, but Rust doesn't speak C++.

* *"Windows, macOS, and Linux all use UTF-16 or UTF-32 at some level."*  Generally, operating systems *do not* check for validity of strings.  This is why `OsStr` exists: just because it *says* "UTF-16" or "UTF-32", that doesn't mean you'll *actually* get valid text.  This is also why this library assumes all UTF-* text is potentially invalid until exhaustively proven otherwise.
*/