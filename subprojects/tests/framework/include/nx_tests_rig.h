#pragma once

/**
 * @brief The directory the test rig keeps its files in.
 *
 * Shared by the runner and by every suite, because both put things here and both have to agree
 * where: the runner writes the programs it receives and the log of the run, and a suite writes its
 * own report beside them.
 *
 * It belongs to the rig rather than to what fills it. Which directory a received program lands in
 * is this program's policy, so the netloader is handed it at startup rather than having it baked in,
 * and the framework crate is told it the same way. This header is where the rig says it once, so
 * that the runner and the suites cannot answer differently.
 */
#define RIG_DIR "sdmc:/switch/nx-tests"
