#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>
#include <elf.h>
#include "qemu-plugin.h"

QEMU_PLUGIN_EXPORT int qemu_plugin_version = QEMU_PLUGIN_VERSION;

static uint64_t insn_count;
static uint64_t insn_limit;
static bool limit_reached;
static uint64_t main_offset;   // main address from file (or offset for PIE)
static uint64_t entry_offset;  // entry point from file
static bool is_pie;
static bool need_base;         // waiting to determine runtime base
static uint64_t runtime_base;
static uint64_t start_addr;
static bool counting;
static bool count_from_start;  // if true, count from _start instead of main

// Syscall tracking
static uint64_t syscall_count;
static uint64_t syscall_cost;  // Virtual instruction cost per syscall (0 = disabled)

// Track counts for common syscalls (x86_64 syscall numbers)
#define MAX_TRACKED_SYSCALLS 512
static uint64_t syscall_counts[MAX_TRACKED_SYSCALLS];

// Guest memory tracking (actual guest allocations via syscalls)
static uint64_t guest_mmap_bytes = 0;      // Current mmap'd memory
static uint64_t guest_mmap_peak = 0;       // Peak mmap'd memory
static uint64_t guest_brk_base = 0;        // Initial brk (heap start)
static uint64_t guest_brk_current = 0;     // Current brk (heap end)
static bool guest_brk_initialized = false;

// x86_64 syscall names (complete list)
static const char* syscall_names[] = {
    [0] = "read", [1] = "write", [2] = "open", [3] = "close", [4] = "stat",
    [5] = "fstat", [6] = "lstat", [7] = "poll", [8] = "lseek", [9] = "mmap",
    [10] = "mprotect", [11] = "munmap", [12] = "brk", [13] = "rt_sigaction",
    [14] = "rt_sigprocmask", [15] = "rt_sigreturn", [16] = "ioctl", [17] = "pread64",
    [18] = "pwrite64", [19] = "readv", [20] = "writev", [21] = "access", [22] = "pipe",
    [23] = "select", [24] = "sched_yield", [25] = "mremap", [26] = "msync",
    [27] = "mincore", [28] = "madvise", [29] = "shmget", [30] = "shmat",
    [31] = "shmctl", [32] = "dup", [33] = "dup2", [34] = "pause", [35] = "nanosleep",
    [36] = "getitimer", [37] = "alarm", [38] = "setitimer", [39] = "getpid",
    [40] = "sendfile", [41] = "socket", [42] = "connect", [43] = "accept",
    [44] = "sendto", [45] = "recvfrom", [46] = "sendmsg", [47] = "recvmsg",
    [48] = "shutdown", [49] = "bind", [50] = "listen", [51] = "getsockname",
    [52] = "getpeername", [53] = "socketpair", [54] = "setsockopt", [55] = "getsockopt",
    [56] = "clone", [57] = "fork", [58] = "vfork", [59] = "execve", [60] = "exit",
    [61] = "wait4", [62] = "kill", [63] = "uname", [64] = "semget", [65] = "semop",
    [66] = "semctl", [67] = "shmdt", [68] = "msgget", [69] = "msgsnd", [70] = "msgrcv",
    [71] = "msgctl", [72] = "fcntl", [73] = "flock", [74] = "fsync", [75] = "fdatasync",
    [76] = "truncate", [77] = "ftruncate", [78] = "getdents", [79] = "getcwd",
    [80] = "chdir", [81] = "fchdir", [82] = "rename", [83] = "mkdir", [84] = "rmdir",
    [85] = "creat", [86] = "link", [87] = "unlink", [88] = "symlink", [89] = "readlink",
    [90] = "chmod", [91] = "fchmod", [92] = "chown", [93] = "fchown", [94] = "lchown",
    [95] = "umask", [96] = "gettimeofday", [97] = "getrlimit", [98] = "getrusage",
    [99] = "sysinfo", [100] = "times", [101] = "ptrace", [102] = "getuid",
    [103] = "syslog", [104] = "getgid", [105] = "setuid", [106] = "setgid",
    [107] = "geteuid", [108] = "getegid", [109] = "setpgid", [110] = "getppid",
    [111] = "getpgrp", [112] = "setsid", [113] = "setreuid", [114] = "setregid",
    [115] = "getgroups", [116] = "setgroups", [117] = "setresuid", [118] = "getresuid",
    [119] = "setresgid", [120] = "getresgid", [121] = "getpgid", [122] = "setfsuid",
    [123] = "setfsgid", [124] = "getsid", [125] = "capget", [126] = "capset",
    [127] = "rt_sigpending", [128] = "rt_sigtimedwait", [129] = "rt_sigqueueinfo",
    [130] = "rt_sigsuspend", [131] = "sigaltstack", [132] = "utime", [133] = "mknod",
    [134] = "uselib", [135] = "personality", [136] = "ustat", [137] = "statfs",
    [138] = "fstatfs", [139] = "sysfs", [140] = "getpriority", [141] = "setpriority",
    [142] = "sched_setparam", [143] = "sched_getparam", [144] = "sched_setscheduler",
    [145] = "sched_getscheduler", [146] = "sched_get_priority_max",
    [147] = "sched_get_priority_min", [148] = "sched_rr_get_interval", [149] = "mlock",
    [150] = "munlock", [151] = "mlockall", [152] = "munlockall", [153] = "vhangup",
    [154] = "modify_ldt", [155] = "pivot_root", [156] = "_sysctl", [157] = "prctl",
    [158] = "arch_prctl", [159] = "adjtimex", [160] = "setrlimit", [161] = "chroot",
    [162] = "sync", [163] = "acct", [164] = "settimeofday", [165] = "mount",
    [166] = "umount2", [167] = "swapon", [168] = "swapoff", [169] = "reboot",
    [170] = "sethostname", [171] = "setdomainname", [172] = "iopl", [173] = "ioperm",
    [174] = "create_module", [175] = "init_module", [176] = "delete_module",
    [177] = "get_kernel_syms", [178] = "query_module", [179] = "quotactl",
    [180] = "nfsservctl", [181] = "getpmsg", [182] = "putpmsg", [183] = "afs_syscall",
    [184] = "tuxcall", [185] = "security", [186] = "gettid", [187] = "readahead",
    [188] = "setxattr", [189] = "lsetxattr", [190] = "fsetxattr", [191] = "getxattr",
    [192] = "lgetxattr", [193] = "fgetxattr", [194] = "listxattr", [195] = "llistxattr",
    [196] = "flistxattr", [197] = "removexattr", [198] = "lremovexattr",
    [199] = "fremovexattr", [200] = "tkill", [201] = "time", [202] = "futex",
    [203] = "sched_setaffinity", [204] = "sched_getaffinity", [205] = "set_thread_area",
    [206] = "io_setup", [207] = "io_destroy", [208] = "io_getevents", [209] = "io_submit",
    [210] = "io_cancel", [211] = "get_thread_area", [212] = "lookup_dcookie",
    [213] = "epoll_create", [214] = "epoll_ctl_old", [215] = "epoll_wait_old",
    [216] = "remap_file_pages", [217] = "getdents64", [218] = "set_tid_address",
    [219] = "restart_syscall", [220] = "semtimedop", [221] = "fadvise64",
    [222] = "timer_create", [223] = "timer_settime", [224] = "timer_gettime",
    [225] = "timer_getoverrun", [226] = "timer_delete", [227] = "clock_settime",
    [228] = "clock_gettime", [229] = "clock_getres", [230] = "clock_nanosleep",
    [231] = "exit_group", [232] = "epoll_wait", [233] = "epoll_ctl", [234] = "tgkill",
    [235] = "utimes", [236] = "vserver", [237] = "mbind", [238] = "set_mempolicy",
    [239] = "get_mempolicy", [240] = "mq_open", [241] = "mq_unlink", [242] = "mq_timedsend",
    [243] = "mq_timedreceive", [244] = "mq_notify", [245] = "mq_getsetattr",
    [246] = "kexec_load", [247] = "waitid", [248] = "add_key", [249] = "request_key",
    [250] = "keyctl", [251] = "ioprio_set", [252] = "ioprio_get", [253] = "inotify_init",
    [254] = "inotify_add_watch", [255] = "inotify_rm_watch", [256] = "migrate_pages",
    [257] = "openat", [258] = "mkdirat", [259] = "mknodat", [260] = "fchownat",
    [261] = "futimesat", [262] = "newfstatat", [263] = "unlinkat", [264] = "renameat",
    [265] = "linkat", [266] = "symlinkat", [267] = "readlinkat", [268] = "fchmodat",
    [269] = "faccessat", [270] = "pselect6", [271] = "ppoll", [272] = "unshare",
    [273] = "set_robust_list", [274] = "get_robust_list", [275] = "splice", [276] = "tee",
    [277] = "sync_file_range", [278] = "vmsplice", [279] = "move_pages", [280] = "utimensat",
    [281] = "epoll_pwait", [282] = "signalfd", [283] = "timerfd_create", [284] = "eventfd",
    [285] = "fallocate", [286] = "timerfd_settime", [287] = "timerfd_gettime",
    [288] = "accept4", [289] = "signalfd4", [290] = "eventfd2", [291] = "epoll_create1",
    [292] = "dup3", [293] = "pipe2", [294] = "inotify_init1", [295] = "preadv",
    [296] = "pwritev", [297] = "rt_tgsigqueueinfo", [298] = "perf_event_open",
    [299] = "recvmmsg", [300] = "fanotify_init", [301] = "fanotify_mark",
    [302] = "prlimit64", [303] = "name_to_handle_at", [304] = "open_by_handle_at",
    [305] = "clock_adjtime", [306] = "syncfs", [307] = "sendmmsg", [308] = "setns",
    [309] = "getcpu", [310] = "process_vm_readv", [311] = "process_vm_writev",
    [312] = "kcmp", [313] = "finit_module", [314] = "sched_setattr", [315] = "sched_getattr",
    [316] = "renameat2", [317] = "seccomp", [318] = "getrandom", [319] = "memfd_create",
    [320] = "kexec_file_load", [321] = "bpf", [322] = "execveat", [323] = "userfaultfd",
    [324] = "membarrier", [325] = "mlock2", [326] = "copy_file_range", [327] = "preadv2",
    [328] = "pwritev2", [329] = "pkey_mprotect", [330] = "pkey_alloc", [331] = "pkey_free",
    [332] = "statx", [333] = "io_pgetevents", [334] = "rseq", [335] = "pidfd_send_signal",
    [424] = "pidfd_open", [425] = "clone3", [434] = "pidfd_getfd", [435] = "memfd_secret",
    [437] = "epoll_pwait2", [439] = "futex_waitv", [448] = "set_mempolicy_home_node",
    [449] = "cachestat", [450] = "fchmodat2", [451] = "map_shadow_stack",
    [452] = "futex_wake", [453] = "futex_wait", [454] = "futex_requeue",
};
#define NUM_SYSCALL_NAMES (sizeof(syscall_names) / sizeof(syscall_names[0]))

static const char* syscall_name(int64_t num) {
    if (num >= 0 && (size_t)num < NUM_SYSCALL_NAMES && syscall_names[num]) {
        return syscall_names[num];
    }
    return NULL;
}

static void parse_elf(const char *path)
{
    FILE *f = fopen(path, "rb");
    if (!f) return;

    Elf64_Ehdr ehdr;
    if (fread(&ehdr, sizeof(ehdr), 1, f) != 1) goto fail;
    if (memcmp(ehdr.e_ident, ELFMAG, SELFMAG) != 0) goto fail;

    entry_offset = ehdr.e_entry;
    is_pie = (ehdr.e_type == ET_DYN);

    // Find section header string table
    if (ehdr.e_shoff == 0 || ehdr.e_shstrndx == 0) goto use_entry;

    Elf64_Shdr shstrtab_hdr;
    fseek(f, ehdr.e_shoff + ehdr.e_shstrndx * sizeof(Elf64_Shdr), SEEK_SET);
    if (fread(&shstrtab_hdr, sizeof(shstrtab_hdr), 1, f) != 1) goto use_entry;
    if (shstrtab_hdr.sh_size == 0) goto use_entry;

    char *shstrtab = malloc(shstrtab_hdr.sh_size);
    if (!shstrtab) goto use_entry;
    fseek(f, shstrtab_hdr.sh_offset, SEEK_SET);
    if (fread(shstrtab, shstrtab_hdr.sh_size, 1, f) != 1) { free(shstrtab); goto use_entry; }

    // Search for .symtab and .strtab
    Elf64_Shdr symtab_hdr = {0}, strtab_hdr = {0};
    for (int i = 0; i < ehdr.e_shnum; i++) {
        Elf64_Shdr shdr;
        fseek(f, ehdr.e_shoff + i * sizeof(Elf64_Shdr), SEEK_SET);
        if (fread(&shdr, sizeof(shdr), 1, f) != 1) continue;
        if (shdr.sh_name >= shstrtab_hdr.sh_size) continue;
        const char *name = shstrtab + shdr.sh_name;
        if (strcmp(name, ".symtab") == 0) symtab_hdr = shdr;
        else if (strcmp(name, ".strtab") == 0) strtab_hdr = shdr;
    }
    free(shstrtab);

    if (symtab_hdr.sh_size == 0 || strtab_hdr.sh_size == 0) goto use_entry;

    char *strtab = malloc(strtab_hdr.sh_size);
    if (!strtab) goto use_entry;
    fseek(f, strtab_hdr.sh_offset, SEEK_SET);
    if (fread(strtab, strtab_hdr.sh_size, 1, f) != 1) { free(strtab); goto use_entry; }

    size_t nsyms = symtab_hdr.sh_size / sizeof(Elf64_Sym);
    fseek(f, symtab_hdr.sh_offset, SEEK_SET);
    for (size_t i = 0; i < nsyms; i++) {
        Elf64_Sym sym;
        if (fread(&sym, sizeof(sym), 1, f) != 1) break;
        if (sym.st_name >= strtab_hdr.sh_size) continue;
        const char *name = strtab + sym.st_name;
        if (sym.st_value != 0) {
            if (strcmp(name, "main") == 0) {
                main_offset = sym.st_value;
                break;
            }
            if (strcmp(name, "main.main") == 0) {
                main_offset = sym.st_value;
                break;
            }
        }
    }
    free(strtab);

use_entry:
    if (!main_offset) main_offset = entry_offset;
    fclose(f);
    return;

fail:
    fclose(f);
}

static void vcpu_syscall(qemu_plugin_id_t id, unsigned int vcpu_index,
                         int64_t num, uint64_t a1, uint64_t a2,
                         uint64_t a3, uint64_t a4, uint64_t a5,
                         uint64_t a6, uint64_t a7, uint64_t a8)
{
    // Always count syscalls when from_start mode is enabled
    // (counting should already be true, but syscalls might fire before first TB)
    if (!counting && !count_from_start) return;

    syscall_count++;
    if (num >= 0 && num < MAX_TRACKED_SYSCALLS) {
        syscall_counts[num]++;
    }

    // Track guest memory allocations
    if (num == 9) {  // mmap
        // mmap(addr, length, prot, flags, fd, offset)
        // a2 = length
        uint64_t length = a2;
        guest_mmap_bytes += length;
        if (guest_mmap_bytes > guest_mmap_peak) {
            guest_mmap_peak = guest_mmap_bytes;
        }
    } else if (num == 11) {  // munmap
        // munmap(addr, length) - a2 = length
        uint64_t length = a2;
        if (length <= guest_mmap_bytes) {
            guest_mmap_bytes -= length;
        } else {
            guest_mmap_bytes = 0;
        }
    }

    // Add virtual cost for syscalls if enabled
    if (syscall_cost > 0) {
        insn_count += syscall_cost;
        if (insn_limit && insn_count >= insn_limit) {
            limit_reached = true;
            exit(137);
        }
    }
}

static void vcpu_syscall_ret(qemu_plugin_id_t id, unsigned int vcpu_index,
                              int64_t num, int64_t ret)
{
    // Track brk return values to measure heap growth
    if (num == 12 && ret > 0) {  // brk returns new brk address (or current if arg was 0)
        uint64_t new_brk = (uint64_t)ret;
        if (!guest_brk_initialized) {
            guest_brk_base = new_brk;
            guest_brk_current = new_brk;
            guest_brk_initialized = true;
        } else {
            guest_brk_current = new_brk;
        }
    }
}

static void plugin_exit(qemu_plugin_id_t id, void *p)
{
    uint64_t vm_peak_kb = 0;
    uint64_t vm_rss_kb = 0;
    uint64_t vm_hwm_kb = 0;
    uint64_t vm_data_kb = 0;
    uint64_t vm_stk_kb = 0;

    FILE *status = fopen("/proc/self/status", "r");
    if (status) {
        char line[256];
        while (fgets(line, sizeof(line), status)) {
            if (strncmp(line, "VmPeak:", 7) == 0) {
                sscanf(line + 7, "%" SCNu64, &vm_peak_kb);
            } else if (strncmp(line, "VmRSS:", 6) == 0) {
                sscanf(line + 6, "%" SCNu64, &vm_rss_kb);
            } else if (strncmp(line, "VmHWM:", 6) == 0) {
                sscanf(line + 6, "%" SCNu64, &vm_hwm_kb);
            } else if (strncmp(line, "VmData:", 7) == 0) {
                sscanf(line + 7, "%" SCNu64, &vm_data_kb);
            } else if (strncmp(line, "VmStk:", 6) == 0) {
                sscanf(line + 6, "%" SCNu64, &vm_stk_kb);
            }
        }
        fclose(status);
    }

    uint64_t io_read_bytes = 0;
    uint64_t io_write_bytes = 0;

    FILE *io = fopen("/proc/self/io", "r");
    if (io) {
        char line[256];
        while (fgets(line, sizeof(line), io)) {
            if (strncmp(line, "rchar:", 6) == 0) {
                sscanf(line + 6, "%" SCNu64, &io_read_bytes);
            } else if (strncmp(line, "wchar:", 6) == 0) {
                sscanf(line + 6, "%" SCNu64, &io_write_bytes);
            }
        }
        fclose(io);
    }

    // Build syscall breakdown for non-zero counts
    char syscall_breakdown[4096] = "";
    int offset = 0;
    bool first = true;
    for (int i = 0; i < MAX_TRACKED_SYSCALLS && offset < 4000; i++) {
        if (syscall_counts[i] > 0) {
            const char *name = syscall_name(i);
            if (name) {
                offset += snprintf(syscall_breakdown + offset, sizeof(syscall_breakdown) - offset,
                                   "%s\"%s\": %" PRIu64, first ? "" : ", ", name, syscall_counts[i]);
            } else {
                offset += snprintf(syscall_breakdown + offset, sizeof(syscall_breakdown) - offset,
                                   "%s\"sys_%d\": %" PRIu64, first ? "" : ", ", i, syscall_counts[i]);
            }
            first = false;
        }
    }

    // Calculate guest heap size from brk
    uint64_t guest_heap_bytes = 0;
    if (guest_brk_initialized && guest_brk_current > guest_brk_base) {
        guest_heap_bytes = guest_brk_current - guest_brk_base;
    }

    fprintf(stderr, "\n{\"instructions\": %" PRIu64 ", \"memory_peak_kb\": %" PRIu64
            ", \"memory_rss_kb\": %" PRIu64 ", \"memory_hwm_kb\": %" PRIu64
            ", \"memory_data_kb\": %" PRIu64 ", \"memory_stack_kb\": %" PRIu64
            ", \"io_read_bytes\": %" PRIu64 ", \"io_write_bytes\": %" PRIu64
            ", \"guest_mmap_bytes\": %" PRIu64 ", \"guest_mmap_peak\": %" PRIu64
            ", \"guest_heap_bytes\": %" PRIu64
            ", \"limit_reached\": %s, \"syscalls\": %" PRIu64
            ", \"syscall_cost\": %" PRIu64
            ", \"syscall_breakdown\": {%s}}\n",
            insn_count, vm_peak_kb, vm_rss_kb, vm_hwm_kb, vm_data_kb, vm_stk_kb,
            io_read_bytes, io_write_bytes, guest_mmap_bytes, guest_mmap_peak,
            guest_heap_bytes, limit_reached ? "true" : "false",
            syscall_count, syscall_cost, syscall_breakdown);
}

static void vcpu_tb_exec(unsigned int cpu_index, void *udata)
{
    uint64_t n = (uint64_t)udata;
    insn_count += n;
    if (insn_limit && insn_count >= insn_limit) {
        limit_reached = true;
        exit(137);
    }
}

static void vcpu_tb_trans(qemu_plugin_id_t id, struct qemu_plugin_tb *tb)
{
    uint64_t addr = qemu_plugin_tb_vaddr(tb);
    size_t n = qemu_plugin_tb_n_insns(tb);

    // Determine runtime base from first TB if needed
    if (need_base) {
        // First TB should be near the entry point
        // For PIE: runtime_entry = addr (approximately), base = addr - entry_offset
        runtime_base = addr - entry_offset;
        start_addr = runtime_base + main_offset;
        need_base = false;
    }

    if (!counting) {
        // Check if any instruction in this TB is at start_addr
        for (size_t i = 0; i < n; i++) {
            struct qemu_plugin_insn *insn = qemu_plugin_tb_get_insn(tb, i);
            if (qemu_plugin_insn_vaddr(insn) == start_addr) {
                counting = true;
                break;
            }
        }
        if (!counting) return;
    }

    qemu_plugin_register_vcpu_tb_exec_cb(tb, vcpu_tb_exec,
                                         QEMU_PLUGIN_CB_NO_REGS, (void *)n);
}

QEMU_PLUGIN_EXPORT int qemu_plugin_install(qemu_plugin_id_t id,
                                           const qemu_info_t *info,
                                           int argc, char **argv)
{
    for (int i = 0; i < argc; i++) {
        char *p = argv[i];
        if (strncmp(p, "limit=", 6) == 0) {
            insn_limit = strtoull(p + 6, NULL, 10);
        } else if (strncmp(p, "binary=", 7) == 0) {
            parse_elf(p + 7);
        } else if (strncmp(p, "syscall_cost=", 13) == 0) {
            syscall_cost = strtoull(p + 13, NULL, 10);
        } else if (strcmp(p, "from_start") == 0 || strcmp(p, "from_start=true") == 0 || strcmp(p, "from_start=on") == 0) {
            count_from_start = true;
        }
    }

    if (count_from_start) {
        // Count from very first instruction - captures ALL user-space instructions
        counting = true;
    } else if (main_offset == 0) {
        counting = true;  // No binary or couldn't parse, count everything
    } else if (is_pie) {
        need_base = true;  // Wait for first TB to determine base
    } else {
        start_addr = main_offset;  // Non-PIE: use address directly
    }

    qemu_plugin_register_vcpu_tb_trans_cb(id, vcpu_tb_trans);
    qemu_plugin_register_vcpu_syscall_cb(id, vcpu_syscall);
    qemu_plugin_register_vcpu_syscall_ret_cb(id, vcpu_syscall_ret);
    qemu_plugin_register_atexit_cb(id, plugin_exit, NULL);
    return 0;
}
