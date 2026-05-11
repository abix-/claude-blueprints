---
name: assembly
description: x86-64 assembly for reverse engineering game binaries, reading IDA/Ghidra/Cutter output, computing image-relative offsets, and writing inline asm or shellcode for UE4SS Rust mods. Use when working with disassembly, RVAs, or low-level memory layout.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# x86-64 Assembly

Focus: reading disassembly to find functions, structs, and offsets in game
binaries. Not authoring full asm modules. Most of this skill is about NOT
guessing what an instruction means.

## Calling conventions
- **Windows x64 (MSVC, all game targets here):**
  - Integer/pointer args: `RCX, RDX, R8, R9`, then stack.
  - Float args: `XMM0..XMM3`.
  - Return: `RAX` (int/ptr) or `XMM0` (float).
  - Caller allocates 32 bytes of shadow space on the stack for the callee, even with <4 args.
  - `this` pointer is in `RCX` for `__thiscall` member functions.
  - Non-volatile (callee-saved): `RBX, RBP, RDI, RSI, R12-R15, XMM6-XMM15`.
  - Volatile (caller-saved): `RAX, RCX, RDX, R8-R11, XMM0-XMM5`.
- **System V (Linux/macOS, not relevant for game mods here but appears in toolchain code):** args `RDI, RSI, RDX, RCX, R8, R9`, no shadow space.

## Image-relative offsets (RVA)
- Game functions are recorded as `RVA = absolute_address - image_base`.
- At runtime, image base is whatever Windows loaded the exe at (ASLR). Resolve with `GetModuleHandle(NULL) + RVA`.
- IDA shows RVAs by default when the image base is set to 0. Ghidra shows them as `image-base + offset`.
- Cross-reference IDA RVAs to the live process: `image_base + (ida_addr - ida_image_base)`.

## Reading common patterns
- `mov rcx, [rcx + 0x40]` -> follow field at offset 0x40 in the object pointed to by `this`.
- `lea rax, [rip + 0x12345]` -> RIP-relative addressing. `rax = next_instruction_address + 0x12345`. Used for globals, vtables, string literals.
- `call qword ptr [rax + 0x28]` -> virtual call. The vtable lives at `*rax`, and slot `0x28 / 8 = 5` is the method.
- `mov rax, [rax]` followed by `call qword ptr [rax + N]` is the classic vtable dispatch sequence.
- `xor eax, eax` -> zero `rax` (writing eax zeros the upper 32 bits). Faster than `mov rax, 0`.
- `test eax, eax / jz` -> branch if `eax == 0`. Standard null/zero check.

## Struct layout
- Field offset = byte distance from the start of the struct. `[rcx + 0x40]` accesses the field at offset `0x40`.
- 8-byte alignment is default for x64. Pointers, `int64`, and `double` align to 8. `bool` is 1 but padded.
- C++ vtable pointer is at offset 0x0 of any class with virtual methods.
- Inheritance: derived class layout starts with the base class layout. `ASurvivalCharacter` starts with `ACharacter` fields, etc.

## Function prologue / epilogue
- Standard MSVC prologue: `push rbp; mov rbp, rsp; sub rsp, N`. Some leaf functions omit `rbp`.
- `sub rsp, 0x28` is the minimum for a non-leaf: 0x20 shadow + 0x08 alignment.
- Epilogue: `add rsp, N; pop rbp; ret`.
- A `ret` with no `add rsp` means the function is a tail call or a thunk.

## Hooking
- 5-byte relative jump: `E9 xx xx xx xx`. `xx xx xx xx = target - (source + 5)`.
- 14-byte absolute jump for >2GB distance: `FF 25 00 00 00 00` followed by 8-byte target.
- Detour stubs (MinHook, Polyhook): overwrite first N bytes with jump to your hook; trampoline executes original bytes then jumps back.
- Always relocate RIP-relative instructions when copying into a trampoline.

## Common reverse-engineering workflow
1. Find function by string xref or vtable slot in IDA.
2. Note RVA. Convert to source-code location via PDB if available, otherwise read the asm.
3. Identify struct offsets by tracing `[reg + N]` accesses. Cross-check across multiple callers.
4. Validate by reading the same memory live (via Cheat Engine, x64dbg, or your mod's runtime control plane).
5. Promote to a Rust struct: `#[repr(C)]` with explicit padding fields if needed. Verify `size_of::<T>()` matches IDA.

## Inline asm in Rust
- `core::arch::asm!` (stable). Always specify clobbers and use `nomem`/`nostack`/`preserves_flags` when applicable.
- Prefer intrinsics (`core::arch::x86_64::*`) over inline asm. They optimize and don't break across LLVM versions.
- For hooks, write the trampoline in a separate `.s` file or use a library. Hand-rolling inline asm for hooks is rarely worth it.

## Tooling
- **IDA Pro / IDA Free:** the standard. Hex-Rays decompiler is worth its weight; trust it for control flow, verify struct offsets in asm.
- **Ghidra:** free, good decompiler. Slower than IDA but the script API is open.
- **x64dbg:** dynamic. Use to validate static analysis against the running process.
- **Cheat Engine:** fastest way to find a field offset when you know the value. Pointer scans are unreliable; combine with disassembly.
- **dumpbin / objdump:** quick header + import table dumps.

## Avoid
- Trusting the decompiler's variable names. They are guesses.
- Assuming a struct layout from one caller. Verify across 3+ uses.
- Hardcoding absolute addresses. Always image-relative.
- Reading volatile registers across function boundaries without saving.
- Writing inline asm when an intrinsic exists.
