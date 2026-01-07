#pragma once

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Opaque struct for the sending-half of a one-shot channel.
typedef struct NxSyncOneshotSender NxSyncOneshotSender;

/// Opaque struct for the receiving-half of a one-shot channel.
typedef struct NxSyncOneshotReceiver NxSyncOneshotReceiver;

/**
 * @brief Creates a new one-shot channel.
 * @param[out] sender Pointer to write the created sender to.
 * @param[out] receiver Pointer to write the created receiver to.
 * @remark The caller is responsible for freeing the sender and receiver with the appropriate `free` functions, unless they are consumed by `send` or `recv`.
 */
void __nx_std_sync__oneshot_create(NxSyncOneshotSender** sender, NxSyncOneshotReceiver** receiver);

/**
 * @brief Frees a `NxSyncOneshotSender`.
 * @param[in] sender The sender to free.
 * @note If `sender` is `NULL`, this function does nothing.
 */
void __nx_std_sync__oneshot_sender_free(NxSyncOneshotSender* sender);

/**
 * @brief Frees a `NxSyncOneshotReceiver`.
 * @param[in] receiver The receiver to free.
 * @note If `receiver` is `NULL`, this function does nothing.
 */
void __nx_std_sync__oneshot_receiver_free(NxSyncOneshotReceiver* receiver);

/**
 * @brief Sends a value on the channel, consuming the sender.
 * @param[in] sender The sender.
 * @param[in] value The value to send (void*).
 * @return 0 on success, -1 on failure (e.g., receiver was dropped).
 * @remark This function takes ownership of the `sender` and it must not be used again, regardless of whether the send was successful.
 */
int32_t __nx_std_sync__oneshot_send(NxSyncOneshotSender* sender, void* value);

/**
 * @brief Receives a value from the channel, consuming the receiver.
 * @param[in] receiver The receiver.
 * @param[out] out_value Pointer to write the received value to.
 * @return 0 on success, -1 on failure (e.g., sender was dropped).
 * @remark This function takes ownership of the `receiver` and it must not be used again, regardless of whether the receive was successful.
 */
int32_t __nx_std_sync__oneshot_recv(NxSyncOneshotReceiver* receiver, void** out_value);

#ifdef __cplusplus
}
#endif
