# RNIDBG

An ARM64 emulator written in Rust, based on the secondary development of unidbg.。[Join us now!](https://discord.gg/MKR2wz863h)

## Build Me

- Make sure your Rust version is 1.79+, otherwise upgrade!
- If Linux, make sure that `libfmt` is available in your environment (**dynarmic backend**).

## DEVELOPER DEBUGGING COMPILE TIME VARIABLES

| 变量名                     | 说明                             | 默认值 |
|-------------------------|--------------------------------|-----|
| PRINT_SYSCALL_LOG       | print syscall log              | 0   |
| SHOW_INIT_FUNC_CALL     | print `init_function` calls    | 0   |
| SHOW_MODULES_INSERT_LOG | print module loading log       | 0   |
| PRINT_SVC_REGISTER      | print service registration log | 0   |
| PRINT_JNI_CALLS         | print jni call log             | 0   |
| DYNARMIC_DEBUG          | print dynarmic logs            | 0   |
| EMU_LOG                 | print emulator logs            | 0   |
| PRINT_MMAP_LOG          | print virtual mmap logs        | 0   |

## RUN TIME VARIABLE IN COMPUTING

| VARIABLE NAME     | CLARIFICATION        | DEFAULT VALUE |
|-------------------|----------------------|---------------|
| DYNARMIC_JIT_SIZE | Code Cache Size (MB) | 64            |

## TODO

- [ ] Add support for debugging
- [ ] Add support for more syscall
- [ ] Beautiful JNI implementation (unsafe block)
- [ ] Implement most system libraries as virtual modules

## Thanks

- [Rust](https://www.rust-lang.org/)
- [unidbg](https://github.com/zhkl0228/unidbg)
- [unidbg-fetch-qsign](https://github.com/fuqiuluo/unidbg-fetch-qsign)
- [dynarmic](https://github.com/lioncash/dynarmic)