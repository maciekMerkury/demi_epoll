#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <sys/epoll.h>
#include <sys/socket.h>

int dpoll_socket(int domain, int type, int proto);

int dpoll_bind(int socket_fd, const sockaddr *addr, socklen_t addr_len);

int dpoll_listen(int socket_fd, int backlog);

int dpoll_accept(int socket_fd, sockaddr *addr, socklen_t *addr_len);

int dpoll_close(int fd);

ssize_t dpoll_write(int socket_fd, const void *buf, size_t len);

ssize_t dpoll_read(int socket_fd, void *buf, size_t len);

ssize_t dpoll_writev(int socket_fd, const iovec *vecs, int iovec_count);

ssize_t dpoll_readv(int socket_fd, iovec *vecs, int iovec_count);

int dpoll_init(void);

int dpoll_create(int flags);

int dpoll_ctl(int dpollfd, int op, int fd, epoll_event *event);

int dpoll_pwait(int dpollfd,
                epoll_event *events,
                int events_len,
                int timeout,
                const sigset_t *sigmask);
