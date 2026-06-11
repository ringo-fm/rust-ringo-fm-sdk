/*
 * Per-slot tool trampoline pool for the Rust SDK.
 *
 * Mirrors bridge.c in go-ringo-fm-sdk: each slot has a distinct C function
 * pointer so the Swift runtime can route tool calls to the correct Rust handler
 * when multiple tools are registered on the same session.
 *
 * rust_fm_tool_callback_slot() is the extern "C" entry point declared in tool.rs
 * and implemented there via Rust.
 */

#include <stdint.h>

typedef const void *FMGeneratedContentRef;
typedef const void *FMGenerationSchemaRef;
typedef const void *FMBridgedToolRef;
typedef void (*FMToolCallable)(FMGeneratedContentRef, unsigned int);

extern FMBridgedToolRef FMBridgedToolCreate(
    const char *name,
    const char *description,
    FMGenerationSchemaRef parameters,
    FMToolCallable callable,
    int *outErrorCode,
    char **outErrorDescription
);

/* Declared in tool.rs with `#[no_mangle] pub extern "C"`. */
extern void rust_fm_tool_callback_slot(int slot, FMGeneratedContentRef content, unsigned int call_id);

#define FM_TOOL_SLOTS 32

#define DEFINE_TOOL_TRAMPOLINE(N) \
    static void fm_rust_tool_trampoline_##N(FMGeneratedContentRef c, unsigned int id) { \
        rust_fm_tool_callback_slot(N, c, id); \
    }

DEFINE_TOOL_TRAMPOLINE(0)
DEFINE_TOOL_TRAMPOLINE(1)
DEFINE_TOOL_TRAMPOLINE(2)
DEFINE_TOOL_TRAMPOLINE(3)
DEFINE_TOOL_TRAMPOLINE(4)
DEFINE_TOOL_TRAMPOLINE(5)
DEFINE_TOOL_TRAMPOLINE(6)
DEFINE_TOOL_TRAMPOLINE(7)
DEFINE_TOOL_TRAMPOLINE(8)
DEFINE_TOOL_TRAMPOLINE(9)
DEFINE_TOOL_TRAMPOLINE(10)
DEFINE_TOOL_TRAMPOLINE(11)
DEFINE_TOOL_TRAMPOLINE(12)
DEFINE_TOOL_TRAMPOLINE(13)
DEFINE_TOOL_TRAMPOLINE(14)
DEFINE_TOOL_TRAMPOLINE(15)
DEFINE_TOOL_TRAMPOLINE(16)
DEFINE_TOOL_TRAMPOLINE(17)
DEFINE_TOOL_TRAMPOLINE(18)
DEFINE_TOOL_TRAMPOLINE(19)
DEFINE_TOOL_TRAMPOLINE(20)
DEFINE_TOOL_TRAMPOLINE(21)
DEFINE_TOOL_TRAMPOLINE(22)
DEFINE_TOOL_TRAMPOLINE(23)
DEFINE_TOOL_TRAMPOLINE(24)
DEFINE_TOOL_TRAMPOLINE(25)
DEFINE_TOOL_TRAMPOLINE(26)
DEFINE_TOOL_TRAMPOLINE(27)
DEFINE_TOOL_TRAMPOLINE(28)
DEFINE_TOOL_TRAMPOLINE(29)
DEFINE_TOOL_TRAMPOLINE(30)
DEFINE_TOOL_TRAMPOLINE(31)

static FMToolCallable fm_rust_tool_trampolines[FM_TOOL_SLOTS] = {
    fm_rust_tool_trampoline_0,  fm_rust_tool_trampoline_1,  fm_rust_tool_trampoline_2,  fm_rust_tool_trampoline_3,
    fm_rust_tool_trampoline_4,  fm_rust_tool_trampoline_5,  fm_rust_tool_trampoline_6,  fm_rust_tool_trampoline_7,
    fm_rust_tool_trampoline_8,  fm_rust_tool_trampoline_9,  fm_rust_tool_trampoline_10, fm_rust_tool_trampoline_11,
    fm_rust_tool_trampoline_12, fm_rust_tool_trampoline_13, fm_rust_tool_trampoline_14, fm_rust_tool_trampoline_15,
    fm_rust_tool_trampoline_16, fm_rust_tool_trampoline_17, fm_rust_tool_trampoline_18, fm_rust_tool_trampoline_19,
    fm_rust_tool_trampoline_20, fm_rust_tool_trampoline_21, fm_rust_tool_trampoline_22, fm_rust_tool_trampoline_23,
    fm_rust_tool_trampoline_24, fm_rust_tool_trampoline_25, fm_rust_tool_trampoline_26, fm_rust_tool_trampoline_27,
    fm_rust_tool_trampoline_28, fm_rust_tool_trampoline_29, fm_rust_tool_trampoline_30, fm_rust_tool_trampoline_31,
};

FMBridgedToolRef fm_rust_tool_create_at_slot(
    int slot,
    const char *name,
    const char *description,
    FMGenerationSchemaRef parameters,
    int *out_error_code,
    char **out_error_description
) {
    if (slot < 0 || slot >= FM_TOOL_SLOTS) {
        if (out_error_code) *out_error_code = 0xFF;
        return (FMBridgedToolRef)0;
    }
    return FMBridgedToolCreate(
        name, description, parameters,
        fm_rust_tool_trampolines[slot],
        out_error_code, out_error_description
    );
}
