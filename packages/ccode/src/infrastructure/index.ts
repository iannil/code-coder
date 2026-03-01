/**
 * Infrastructure Module
 *
 * Core infrastructure components for the ccode system:
 * - Redis Streams for task queue and event sourcing
 */

export {
  RedisStreamClient,
  RedisStreamConfig,
  StreamMessage,
  PendingMessage,
  streamKeys,
  getRedisStreamClient,
  closeRedisStreamClient,
  isRedisStreamsAvailable,
} from "./redis"
