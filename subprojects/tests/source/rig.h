#pragma once

/**
 * @brief The directory the test rig keeps its files in.
 *
 * Shared by the runner and by every suite, because both put things here and both have to agree
 * where: the runner writes the programs it receives and the log of the run, and a suite writes its
 * own report beside them.
 *
 * It belongs to the rig rather than to the netloader that fills it. Which directory a received
 * program lands in is this program's policy, so it is handed to the netloader at startup instead of
 * being baked into it.
 */
#define RIG_DIR "sdmc:/switch/nx-tests"
