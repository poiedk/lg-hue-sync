/*
 * Compatibility shims for older LG webOS libc builds.
 *
 * webOS 3.x-era firmware predates several libc wrappers referenced by modern
 * Rust std/dependencies. Keep this file narrowly scoped to symbols that can be
 * implemented against kernel interfaces available on the target.
 */
#include <stdio.h>
#include <unistd.h>
#include <sys/syscall.h>

unsigned long getauxval(unsigned long type) {
    struct {
        unsigned long a_type;
        unsigned long a_val;
    } aux;
    unsigned long result = 0;

    FILE *file = fopen("/proc/self/auxv", "rb");
    if (!file) {
        return 0;
    }

    while (fread(&aux, sizeof(aux), 1, file) == 1) {
        if (aux.a_type == 0) {
            break;
        }
        if (aux.a_type == type) {
            result = aux.a_val;
            break;
        }
    }

    fclose(file);
    return result;
}

int gettid(void) {
    return (int)syscall(SYS_gettid);
}

/*
 * Avoid depending on a libc declaration of struct mmsghdr: syscall(2) forwards
 * this pointer unchanged and the shim never dereferences it.
 */
int sendmmsg(int sockfd, void *msgvec, unsigned int vlen, int flags) {
    return (int)syscall(SYS_sendmmsg, sockfd, msgvec, vlen, flags);
}
