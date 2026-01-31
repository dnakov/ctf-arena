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

static void plugin_exit(qemu_plugin_id_t id, void *p)
{
    FILE *status = fopen("/proc/self/status", "r");
    uint64_t vm_peak_kb = 0;
    if (status) {
        char line[256];
        while (fgets(line, sizeof(line), status)) {
            if (strncmp(line, "VmPeak:", 7) == 0) {
                sscanf(line + 7, "%" SCNu64, &vm_peak_kb);
                break;
            }
        }
        fclose(status);
    }
    fprintf(stderr, "\n{\"instructions\": %" PRIu64 ", \"memory_peak_kb\": %" PRIu64 ", \"limit_reached\": %s}\n",
            insn_count, vm_peak_kb, limit_reached ? "true" : "false");
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
        }
    }

    if (main_offset == 0) {
        counting = true;  // No binary or couldn't parse, count everything
    } else if (is_pie) {
        need_base = true;  // Wait for first TB to determine base
    } else {
        start_addr = main_offset;  // Non-PIE: use address directly
    }

    qemu_plugin_register_vcpu_tb_trans_cb(id, vcpu_tb_trans);
    qemu_plugin_register_atexit_cb(id, plugin_exit, NULL);
    return 0;
}
