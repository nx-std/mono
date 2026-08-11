#include <netinet/in.h>
#include <stdbool.h>
#include <string.h>
#include <sys/socket.h>

#include "../../harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0011: getpeername reports the connected peer">

test_rc_t test_0011_getpeername_reports_the_connected_peer(void)
{
    //* Given
    // A connected pair, so each end has a peer to report.
    int client = -1;
    int server = -1;
    if (net_connected_pair(NET_TEST_PORT, &client, &server) != 0) {
        return TEST_SETUP_FAILED;
    }

    //* When
    // Each end is asked both who it is and who it is talking to.
    struct sockaddr_in client_self;
    struct sockaddr_in server_peer;
    struct sockaddr_in client_peer;
    memset(&client_self, 0, sizeof(client_self));
    memset(&server_peer, 0, sizeof(server_peer));
    memset(&client_peer, 0, sizeof(client_peer));
    socklen_t client_self_len = sizeof(client_self);
    socklen_t server_peer_len = sizeof(server_peer);
    socklen_t client_peer_len = sizeof(client_peer);

    const int named_client = getsockname(client, (struct sockaddr*)&client_self, &client_self_len);
    const int server_named_peer =
        getpeername(server, (struct sockaddr*)&server_peer, &server_peer_len);
    const int client_named_peer =
        getpeername(client, (struct sockaddr*)&client_peer, &client_peer_len);

    //* Then
    // The answers cross: what the server calls its peer is what the client
    // calls itself, and the client's peer is the port the listener was bound
    // to. Either side alone would pass on an implementation that reported the
    // socket's own address for both.
    const bool correct = named_client == 0
        && server_named_peer == 0
        && client_named_peer == 0
        && server_peer.sin_port == client_self.sin_port
        && server_peer.sin_addr.s_addr == client_self.sin_addr.s_addr
        && ntohs(client_peer.sin_port) == NET_TEST_PORT;

    net_close(client);
    net_close(server);
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
