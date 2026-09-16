// CI-only loader shim: reproduce NoWaylandLib without uninstalling host libraries.
#define _GNU_SOURCE
#include <dlfcn.h>
#include <string.h>
void *dlopen(const char *name, int flags) {
    void *(*real_dlopen)(const char *, int) = dlsym(RTLD_NEXT, "dlopen");
    if (name && strstr(name, "libwayland-client.so")) return NULL;
    return real_dlopen(name, flags);
}
