# Token Tracing

Token Tracing is the domain of deriving privacy-safe token-usage totals from local coding-agent session data and presenting a current aggregate without retaining conversational content.

## Language

**Provider**:
A supported coding-agent product that produces local session data, currently Claude Code or Codex.
_Avoid_: Agent, integration

**Source**:
The user-enabled local session data belonging to one Provider.
_Avoid_: Installation, feed

**Source Root**:
The configured boundary within which a Source may be discovered and read.
_Avoid_: Scan path, home directory

**Session**:
One opaque Provider-defined span of related coding-agent activity.
_Avoid_: Conversation, chat

**Observation**:
Privacy-safe token metadata derived from one Provider record.
_Avoid_: Message, raw record

**Incremental Observation**:
An Observation whose token values represent new usage by themselves.
_Avoid_: Event total

**Cumulative Observation**:
An Observation whose token values represent usage accumulated up to that point.
_Avoid_: Delta

**Usage Event**:
A deduplicated token delta accepted into Token Tracing totals.
_Avoid_: Observation, raw event

**Monotonic Segment**:
A consecutive span in which a cumulative token counter does not decrease.
_Avoid_: Session, reset

**Checkpoint**:
A restart-safe collection position describing how much of a Source has already been processed.
_Avoid_: Bookmark, cursor

**Active Provider**:
The Provider with the newest valid Usage Event inside the activity window.
_Avoid_: Current agent, selected provider

**Current-session Total**:
The sum of accepted Usage Events belonging to the latest active Session.
_Avoid_: Current tokens, conversation total

**Today's Total**:
The sum of accepted Usage Events within the current Windows local calendar day across enabled Providers.
_Avoid_: Daily usage, last 24 hours

**Source Health**:
The current ability to collect a Provider's configured Source independently of other Providers.
_Avoid_: App status, connection status

**Usage Summary**:
The privacy-safe aggregate presented to the overlay: activity state, optional Provider, current-session total, Today's Total, last update, and Source Health.
_Avoid_: Dashboard data, raw usage
