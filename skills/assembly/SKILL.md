---
name: assembly
description: x86-64 assembly for reverse engineering game binaries, reading IDA/Ghidra/Cutter output, computing image-relative offsets, and writing inline asm or shellcode for UE4SS Rust mods. Built from the Intel SDM, MS x64 ABI doc, Agner Fog's optimization manuals, and game-modding community practice.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# x86-64 Assembly

Focus: reading disassembly to find functions, structs, and offsets in
game binaries; writing the occasional hook trampoline; sanity-checking
LLVM/MSVC output. Not authoring full asm modules.

This skill is mostly about **NOT guessing**: misreading one
instruction or addressing mode produces wrong offsets, which produce
broken mods.

## Authoritative references

- **Intel 64 SDM Vol 2** (instruction reference). The one source of
  truth for what an opcode does.
- **Microsoft x64 software conventions** (calling convention, stack
  unwind, exception handling).
- **Agner Fog's manuals** (optimizing assembly, instruction tables,
  microarchitecture). Free, updated regularly.
- **uops.info** for per-uop latency / throughput on modern
  microarchitectures.
- For ARM64 (Apple silicon / mobile): different ISA, different ABI;
  this skill is x86-64 only.

## Calling conventions

### Windows x64 (MSVC)

All Windows game targets. Applies to UE4SS Rust mods, hook bodies,
and any code calling into native game functions.

| Register | Role |
|----------|------|
| `RCX`, `RDX`, `R8`, `R9` | Integer / pointer args 1-4. |
| `XMM0`, `XMM1`, `XMM2`, `XMM3` | Float / vector args 1-4. Arg N uses the Nth register from either set (positional). |
| Stack (RSP+8 onward) | Args 5+. |
| `RAX` | Integer / pointer return. |
| `XMM0` | Float / vector return. |
| `RCX` | `this` pointer (member functions). |
| `RBX, RBP, RDI, RSI, R12-R15, XMM6-XMM15` | Non-volatile (callee-saved). |
| `RAX, RCX, RDX, R8-R11, XMM0-XMM5` | Volatile (caller-saved). |

- **Caller allocates 32 bytes of shadow space** below RSP before
  every call, even for functions taking <4 args. Failing to do this
  corrupts the callee's saved args.
- **16-byte stack alignment required at call sites.** `sub rsp, 0x28`
  is the standard non-leaf prologue: 0x20 shadow + 0x08 to align
  (the call instruction itself pushed 8 bytes).
- **Structs >8 bytes are passed by pointer**, not by value (Win64
  detail; differs from System V).
- **Variadic `...` functions** put float args in BOTH the integer
  register AND the corresponding XMM (one of the rare cases where
  registers overlap).

### System V (Linux / macOS Intel)

Game mods don't see this often, but toolchain output does.

| Register | Role |
|----------|------|
| `RDI, RSI, RDX, RCX, R8, R9` | Integer args 1-6. |
| `XMM0-XMM7` | Float args 1-8. |
| No shadow space. |
| Stack alignment: 16 bytes at call sites. |
| Non-volatile: `RBX, RBP, R12-R15`. |
| Volatile: most others. |

## Addressing modes

The general form: `[base + index*scale + disp]`.

| Form | Meaning |
|------|---------|
| `[rcx]` | Direct memory at `rcx`. |
| `[rcx + 0x40]` | `rcx + 0x40` (field at offset 0x40). |
| `[rcx + rdx*8]` | Array indexing, 8-byte elements. |
| `[rcx + rdx*8 + 0x20]` | Field at offset 0x20 of array element. |
| `[rip + 0x12345]` | RIP-relative. `addr = next_instruction + 0x12345`. |
| `qword ptr [rax]` | 8-byte read. Size prefix matters for ambiguous forms. |

- **`scale`** must be 1, 2, 4, or 8.
- **RIP-relative is the default for globals** in 64-bit MSVC/clang.
  `lea rax, [rip + offset]` computes the address; the actual value
  in the encoding is the offset, not the address. Disassemblers
  show the resolved address.
- **`lea` does not access memory.** `lea rax, [rcx + 0x10]` is just
  `rax = rcx + 0x10`. Often used for fast arithmetic.

## Reading common patterns

| Pattern | Meaning |
|---------|---------|
| `mov rcx, [rcx + 0x40]` | Load field at offset 0x40 from object in `rcx`. |
| `lea rax, [rip + 0x12345]` | Compute address of global / vtable / string. |
| `mov rax, [rcx]; call qword ptr [rax + 0x28]` | Virtual call: load vtable, then call slot `0x28 / 8 = 5`. |
| `mov rax, [rax + 0x10]; jmp rax` | Tail-call through function pointer. |
| `xor eax, eax` | Zero `rax`. Writing `eax` zeros the upper 32 bits. Faster encoding than `mov rax, 0`. |
| `test eax, eax; jz label` | Branch if zero. Standard null/zero check. |
| `cmp dword ptr [rcx+8], 0; je ...` | Read 4-byte field, branch on zero. |
| `int 3` | Breakpoint. Compiler often pads function boundaries with `int 3`. |
| `nop dword ptr [rax+rax]` | Multi-byte NOP. Used for alignment padding. |

## Function prologue / epilogue (MSVC)

```
push rbp                   ; save frame pointer (sometimes omitted)
mov rbp, rsp               ; new frame
sub rsp, 0x40              ; stack frame (multiple of 16, includes shadow space)
; ... body ...
add rsp, 0x40
pop rbp
ret
```

- Leaf functions (no calls) often omit `rbp` and just `sub rsp, N`.
- **`sub rsp, 0x28`**. Minimum non-leaf prologue: 0x20 shadow +
  0x08 alignment.
- **A `ret` with no `add rsp`** = tail call or thunk.
- **Saved non-volatile registers** appear just inside the prologue
  via `push rbx`, `push rdi`, etc. The unwind info uses these.

## Image-relative offsets (RVA)

- **RVA = absolute_address - image_base.** Recorded in IDA / Ghidra
  as the function's stored address.
- **At runtime under ASLR**, the image base is randomized. Resolve
  with `GetModuleHandle(NULL) + RVA` to get the actual address.
- **IDA defaults to showing RVAs** when image base is set to 0
  (recommended for game RE). Ghidra shows them as `image-base +
  offset`; use `Tools -> Function -> Show RVA`.
- **Cross-reference IDA to a live process:**
  `live_addr = live_image_base + (ida_addr - ida_image_base)`.
  The mod's runtime control plane (see the `runtime-control-http`
  skill) should expose `image_base` to make this trivial.

## Struct layout (C/C++)

- **Field offset = byte distance from struct start.** `[rcx + 0x40]`
  is the field at offset 0x40 of the object pointed to by `rcx`.
- **Default alignment is the size of the largest member**, capped
  at 16 bytes (Windows) or 8 bytes (most Unix).
- **C++ classes with virtual methods have a vtable pointer at
  offset 0x0.** Then come fields in declaration order.
- **Inheritance:** derived layout starts with the full base layout.
  `ASurvivalCharacter` starts with `ACharacter` fields, which start
  with `APawn` fields, etc.
- **Multiple inheritance** in C++: each base contributes its layout
  in declaration order; the compiler inserts adjustor thunks at the
  vtable slots that need them.
- **Padding:** the compiler inserts padding to align each field to
  its natural boundary. `struct { char a; int64 b; }` is 16 bytes
  (1 + 7 pad + 8), not 9.
- **Verify with `size_of`** in Rust: write the `#[repr(C)]` struct,
  print the size, compare to IDA's reported `sizeof`.

## Hooking

Three common approaches:

### Inline detour (MinHook, Polyhook, custom)

Overwrite first N bytes of target with a jump to your hook;
relocate the overwritten bytes into a trampoline that ends with a
jump back.

```
target:           E9 ?? ?? ?? ??       ; jump to hook (5 bytes)
                  <pad with int3>      ; if first instruction was longer
hook:             ; your code
                  jmp trampoline       ; or just call trampoline like a function
trampoline:       <relocated original bytes>
                  E9 ?? ?? ?? ??       ; jump back to target+N
```

- **5-byte relative jump (`E9`):** range +/-2GB. For >2GB use
  `FF 25 00 00 00 00` + 8-byte absolute address (14 bytes total).
- **Hot-patch slot:** MSVC `/hotpatch` builds emit a 2-byte NOP at
  the function start specifically for inline patching. Rare in
  retail builds.
- **Relocate RIP-relative instructions** when copying to the
  trampoline. The encoded offset is relative to the original
  location; moving the instruction breaks it. MinHook handles this
  automatically.
- **Atomicity:** patch under suspended threads (`SuspendThread` on
  every thread, patch, `ResumeThread`) to avoid a thread executing
  half-patched code.

### Vtable hook

Replace a function pointer in a vtable. Cheap, surgical, no
trampoline needed.

```cpp
void** vtable = *(void***)obj;
DWORD old_protect;
VirtualProtect(&vtable[5], sizeof(void*), PAGE_READWRITE, &old_protect);
void* original = vtable[5];
vtable[5] = &my_hook;
VirtualProtect(&vtable[5], sizeof(void*), old_protect, &old_protect);
```

- Vtables are read-only memory; `VirtualProtect` to RW first.
- Affects ALL instances using that vtable.
- Save `original` to call through to the real implementation.

### IAT / EAT hook

Patch the Import Address Table to redirect a library call. Used for
intercepting `MessageBox`, file I/O, etc. without touching the
library itself.

## Reverse-engineering workflow

1. **Find the function.**
   - String xref (most common): search for a unique log message,
     follow xref back.
   - Vtable slot: identify the class vtable, count slots.
   - Pattern scan: byte signature of a known prologue.
2. **Note the RVA.** Convert to source location via PDB if you have
   one; otherwise read the asm.
3. **Identify struct offsets.** Trace `[reg + N]` accesses across
   at least 3 callers. A single caller can be ambiguous.
4. **Validate live.** Read the same memory with Cheat Engine,
   x64dbg, or the mod's runtime control plane. Compare to your
   static analysis.
5. **Promote to a Rust struct.** Write `#[repr(C)]` with explicit
   padding fields where needed. `size_of::<T>()` must match IDA's
   sizeof.
6. **Commit empirical findings.** Future-you needs to know which
   offsets are verified and which are guesses.

## Inline asm in Rust

```rust
use core::arch::asm;

unsafe {
    let x: u64;
    asm!(
        "mov {0}, qword ptr [{1} + 0x40]",
        out(reg) x,
        in(reg) obj_ptr,
        options(nostack, preserves_flags, readonly),
    );
}
```

- **`core::arch::asm!`** (stable since Rust 1.59). Don't use the old
  `llvm_asm!`.
- **Options:** `nomem` (no memory access), `nostack` (no stack
  changes), `preserves_flags` (no flag clobber), `readonly` (reads
  memory but doesn't write), `pure`. Specifying them lets LLVM
  optimize better.
- **Clobbers:** list every register you write to that isn't an
  output. `out("rax") _` for "I'll clobber rax but don't care
  about the value."
- **Prefer intrinsics over inline asm:** `core::arch::x86_64::*`
  has wrappers for most useful instructions, and the compiler
  schedules them.
- **For hook bodies**, write a small naked function or a `.s` file.
  Inline asm in normal functions has register-allocation interaction
  that surprises people.

## Performance (when writing asm)

Most asm here is for hooks (cold) or struct access (already
optimized by the compiler). When asm performance does matter:

- **Avoid partial-register writes.** `mov al, 1` keeps the upper
  bits of `rax`; `movzx rax, 1` clears them. Modern CPUs partially
  fix this but still pay a merge cost.
- **`xor reg, reg`** to zero. Recognized by every CPU as a zeroing
  idiom (dependency-breaking, zero-latency on Sandy Bridge+).
- **Prefer 32-bit ops** when the result fits. `mov eax, ebx` zeros
  the upper 32 bits and uses a shorter encoding.
- **Branch prediction:** predicted backward branches default to
  "taken" (loops). Forward branches default to "not taken". Lay out
  hot paths fall-through.
- **`LEA` for arithmetic:** `lea rax, [rcx + rdx*4 + 8]` is one
  3-cycle op; the equivalent `imul + add + add` is three.
- **Avoid `div` / `idiv`.** 20-40 cycles. For division by constant,
  use `imul` with reciprocal (compiler does this automatically).
- **Vectorize with intrinsics, not inline asm.** SSE/AVX intrinsics
  in `core::arch::x86_64` are easier to read and the compiler can
  unroll / mask / merge.
- **Cache lines are 64 bytes.** Hot fields fit in one line; cold
  fields don't share lines with hot ones.
- **Branch-free idioms:** `cmov` (`cmovz`, `cmovnz`, etc.) for
  short conditional assignments. Saves a branch but introduces a
  dependency.
- **Microbenchmark on the actual target CPU.** Different
  microarchitectures (Skylake vs Zen 3 vs Apple M1 emulation)
  behave differently.

## Tooling

- **IDA Pro / IDA Free:** the standard. Hex-Rays decompiler ($) is
  worth its weight; trust it for control flow, verify struct offsets
  in asm. Free version: x86-64 only, no decompiler.
- **Ghidra:** free, good decompiler. Slower than IDA but the script
  API (Python / Java) is fully open. NSA-published.
- **Binary Ninja:** mid-priced, fast decompiler, scriptable. Catching
  up with IDA on x86-64.
- **Cutter:** GUI on top of rizin/radare2. Free.
- **x64dbg:** dynamic analysis. Best free option for stepping
  through a live process on Windows.
- **WinDbg / WinDbg Preview:** Microsoft's debugger. Heavy but the
  PDB symbol support is unmatched on Windows binaries.
- **Cheat Engine:** fastest way to find a field offset when you know
  the runtime value. Pointer scans are unreliable; combine with
  static disassembly.
- **dumpbin / objdump / `llvm-objdump`:** quick PE/ELF header and
  import table dumps from the command line.
- **`godbolt.org` (Compiler Explorer):** paste C/C++/Rust, see the
  assembly. Indispensable for understanding what an optimization
  flag actually produces.

## Avoid

- Trusting decompiler-generated variable names. They are guesses.
- Assuming a struct layout from one caller. Verify across 3+ uses.
- Hardcoding absolute addresses. Always image-relative.
- Reading volatile registers across function boundaries without
  saving them.
- Writing inline asm when an intrinsic exists.
- Skipping `options(...)` on `asm!`. Missing `nostack` /
  `preserves_flags` blocks optimizations.
- Patching code without `VirtualProtect`. .text is read-only.
- Patching code without suspending threads. Race condition; thread
  can execute half-written instructions.
- Assuming structs are tightly packed. The compiler pads aggressively
  for alignment.
- Ignoring the difference between `RVA` and `live address`. ASLR is
  on by default.
