/**
 * Observer Integration Module
 *
 * Provides integration clients for connecting Observer Network
 * components to external systems:
 *
 * - channels-client: IM channels via zero-hub
 * - memory-client: Persistent storage via memory-markdown
 * - agent-client: Agent invocation via session API
 *
 * @module observer/integration
 */

export {
  ChannelsClient,
  getChannelsClient,
  resetChannelsClient,
  createChannelsClient,
  type ChannelType,
  type SendMessageRequest,
  type MessageContent,
  type SendMessageResponse,
  type InlineButton,
  type SendWithButtonsRequest,
  type ChannelsClientConfig,
} from "./channels-client"

export {
  MemoryClient,
  getMemoryClient,
  resetMemoryClient,
  createMemoryClient,
  type ObserverMemoryConfig,
  type ObserverHistoryEntry,
} from "./memory-client"

export {
  AgentClient,
  getAgentClient,
  resetAgentClient,
  createAgentClient,
  type AgentInvocation,
  type AgentResult,
  type AgentClientConfig,
} from "./agent-client"

export {
  ObservationRouter,
  getObservationRouter,
  resetObservationRouter,
  createObservationRouter,
  type RoutingRule,
  type RoutingResult,
  type RouterConfig,
} from "./observation-router"
