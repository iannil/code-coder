/**
 * Infrastructure Module
 *
 * Core infrastructure components for the ccode system:
 * - Redis Streams for task queue and event sourcing
 */

export {
  RedisStreamClient,
  streamKeys,
  getRedisStreamClient,
  closeRedisStreamClient,
  isRedisStreamsAvailable,
} from "./redis"

export type {
  RedisStreamConfig,
  StreamMessage,
  PendingMessage,
} from "./redis"
