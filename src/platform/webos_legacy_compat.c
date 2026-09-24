/*
 * Compatibility shims for older LG webOS libc builds.
 *
 * The current OpenLGTV webOS SDK patched GCC already links its static
 * glibc-polyfills library, which supplies getauxval. Keep this file narrowly
 * scoped to additional wrappers referenced by modern Rust std/dependencies.
 */
#include <unistd.h>
#include <sys/syscall.h>

#ifndef SYS_gettid
# ifdef __NR_gettid
#  define SYS_gettid __NR_gettid
# endif
#endif

#ifndef SYS_sendmmsg
# ifdef __NR_sendmmsg
#  define SYS_sendmmsg __NR_sendmmsg
# endif
#endif

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
