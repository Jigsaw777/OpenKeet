"""Bounded scheduling for rolling speech recognition and provisional answers."""
from __future__ import annotations

from collections import OrderedDict, deque
from dataclasses import dataclass
import json
import re
import threading

DEFAULT_QUESTION_PAUSE = 2.0
MIN_QUESTION_PAUSE = 0.6
MAX_QUESTION_PAUSE = 5.0


@dataclass
class AudioChunk:
    utterance_id: int
    samples: object
    speech_at: float | None
    final: bool


class TranscriptionMailbox:
    """Latest partial wins; completed utterances have priority over queued partials."""
    def __init__(self, max_finals=8):
        if max_finals < 1:
            raise ValueError("The final queue needs at least one slot.")
        self.condition = threading.Condition()
        self.finals = deque(maxlen=max_finals)
        self.partial = None
        self.closed = False
        self.completed_ids = deque(maxlen=128)

    def put(self, chunk: AudioChunk) -> int | None:
        """Return the ID of an evicted final, if any."""
        with self.condition:
            if self.closed or chunk.utterance_id in self.completed_ids:
                return None
            evicted = None
            if chunk.final:
                self.completed_ids.append(chunk.utterance_id)
                if self.partial is not None and self.partial.utterance_id == chunk.utterance_id:
                    self.partial = None
                if len(self.finals) == self.finals.maxlen:
                    evicted = self.finals[0].utterance_id
                self.finals.append(chunk)
            else:
                self.partial = chunk
            self.condition.notify()
            return evicted

    def get(self) -> AudioChunk | None:
        with self.condition:
            while not self.finals and self.partial is None and not self.closed:
                self.condition.wait()
            if self.finals:
                return self.finals.popleft()
            if self.partial is not None:
                chunk, self.partial = self.partial, None
                return chunk
            return None

    def close(self):
        with self.condition:
            self.closed = True
            self.partial = None
            self.condition.notify_all()


@dataclass
class AnswerRequest:
    utterance_id: int
    text: str
    speech_at: float | None
    draft: bool
    revision: int = 0
    segments: tuple[str, ...] = ()


class LiveAnswerPlanner:
    """Bounded early drafts, then debounced revisions; no per-word restart loop."""
    def __init__(self):
        self.clear()

    def clear(self):
        self.partials = OrderedDict()
        self.finals = OrderedDict()
        self.drafted = set()
        self.draft_attempts = {}
        self.retry_after_words = {}
        self.completed = {}
        self.ids = deque()

    def _remember(self, utterance_id):
        if utterance_id not in self.ids:
            self.ids.append(utterance_id)
        while len(self.ids) > 128:
            old = self.ids.popleft()
            self.drafted.discard(old)
            self.draft_attempts.pop(old, None)
            self.retry_after_words.pop(old, None)
            self.completed.pop(old, None)
            self.partials.pop(old, None)
            self.finals.pop(old, None)

    @staticmethod
    def ready_for_draft(text):
        words = re.findall(r"[\w#+]+", text.casefold())
        while words and words[0] in {"so", "okay", "now", "please", "well", "um", "uh"}:
            words.pop(0)
        prompt_words = {"what", "why", "how", "when", "which", "who", "where", "explain", "compare",
                        "describe", "define", "implement", "design", "walk", "tell", "show", "discuss"}
        return len(words) >= 3 and (words[0] in prompt_words or text.rstrip().endswith("?")
                                    or words[:2] in (["can", "you"], ["could", "you"], ["would", "you"]))

    def offer_partial(self, utterance_id, text, speech_at, revision=0, segments=()):
        self._remember(utterance_id)
        if utterance_id in self.completed:
            return
        if utterance_id in self.drafted:
            required = self.retry_after_words.get(utterance_id)
            if required is None or len(text.split()) < required:
                return
            self.drafted.discard(utterance_id)
            self.retry_after_words.pop(utterance_id, None)
        if not self.ready_for_draft(text):
            return
        self.partials[utterance_id] = AnswerRequest(utterance_id, text[:4000], speech_at, True, revision, tuple(segments))
        while len(self.partials) > 4:
            self.partials.popitem(last=False)

    def offer_final(self, utterance_id, text, speech_at, revision=0, segments=()):
        self._remember(utterance_id)
        if revision <= self.completed.get(utterance_id, -1):
            return True
        self.completed[utterance_id] = revision
        self.partials.pop(utterance_id, None)
        self.finals[utterance_id] = AnswerRequest(utterance_id, text[:4000], speech_at, False, revision, tuple(segments))
        dropped = len(self.finals) > 8
        if dropped:
            self.finals.popitem(last=False)
        return not dropped

    def next_request(self):
        if self.finals:
            _, request = self.finals.popitem(last=False)
            return request
        if self.partials:
            _, request = self.partials.popitem(last=True)
            self.drafted.add(request.utterance_id)
            self.draft_attempts[request.utterance_id] = self.draft_attempts.get(request.utterance_id, 0) + 1
            return request
        return None

    def drop_partials(self):
        self.partials.clear()

    def defer(self, request):
        if request.draft:
            return
        existing = self.finals.get(request.utterance_id)
        if existing is None or request.revision > existing.revision:
            self.finals[request.utterance_id] = request

    def note_draft_result(self, request, answer):
        # One bounded retry if the first prediction correctly waited for meaning.
        waiting = answer.strip().casefold().startswith("listening for the question")
        if waiting and self.draft_attempts.get(request.utterance_id, 0) < 2:
            self.retry_after_words[request.utterance_id] = len(request.text.split()) + 4

    @property
    def pending(self):
        return bool(self.partials or self.finals)


@dataclass
class QuestionState:
    question_id: int
    fragments: OrderedDict
    open_utterances: set
    speech_at: float
    revision: int = 0
    submitted_revision: int = -1
    semantically_incomplete: bool = False

    @property
    def text(self):
        return " ".join(self.fragments.values()).strip()


class QuestionBuffer:
    """Pause debounce plus fast continuity cues; the model later validates semantics.

    Capture utterances are not automatically treated as separate questions.
    Rolling revisions replace the same utterance's text instead of duplicating it.
    """
    CONTINUATIONS = ("and ", "also ", "but ", "what if ", "how about ", "without ",
                     "with ", "for example", "assuming ", "using ", "instead ",
                     "especially ", "more specifically", "actually ", "i mean ",
                     "sorry ", "even if ", "not ", "suppose ")
    NEW_QUESTION = ("next question", "different question", "new question", "moving on",
                    "let's switch", "another question", "switching topics")
    UNFINISHED = {"and", "or", "but", "if", "when", "because", "with", "without", "for",
                  "to", "that", "which", "using", "where", "so", "versus", "in", "on",
                  "unless", "assuming", "suppose", "considering", "between"}

    def __init__(self, pause_seconds=DEFAULT_QUESTION_PAUSE):
        self.pause_seconds = pause_seconds
        self.clear()

    def clear(self):
        self.groups = OrderedDict()
        self.mapping = {}
        self.activity = {}
        self.partial_stamps = {}
        self.closed_utterances = deque(maxlen=128)
        self.sequence = 0

    def observe_activity(self, utterance_id, stamp):
        if utterance_id in self.closed_utterances:
            return
        first, last = self.activity.get(utterance_id, (stamp, stamp))
        self.activity[utterance_id] = (min(first, stamp), max(last, stamp))
        if utterance_id in self.mapping:
            state = self.groups.get(self.mapping[utterance_id])
            if state:
                state.speech_at = max(state.speech_at, stamp)

    @staticmethod
    def unfinished(text):
        words = re.findall(r"[\w']+", text.casefold())
        return (not words or words[-1] in QuestionBuffer.UNFINISHED or len(words) < 2
                or " ".join(words) in {"can you", "could you", "tell me", "what if", "how about"})

    def _state_for(self, utterance_id, text, speech_at):
        if utterance_id in self.mapping:
            return self.groups[self.mapping[utterance_id]]
        first = self.activity.get(utterance_id, (speech_at, speech_at))[0]
        latest = next(reversed(self.groups.values())) if self.groups else None
        lowered = text.strip().casefold()
        explicit_new = lowered.startswith(self.NEW_QUESTION)
        gap = first - latest.speech_at if latest else float("inf")
        continuation = lowered.startswith(self.CONTINUATIONS)
        same = bool(latest and not explicit_new and
                    (gap <= max(1.4, self.pause_seconds) or
                     (gap <= 8 and (continuation or self.unfinished(latest.text))) or
                     (gap <= 15 and latest.semantically_incomplete)))
        if same:
            state = latest
        else:
            self.sequence += 1
            state = QuestionState(self.sequence, OrderedDict(), set(), speech_at)
            self.groups[state.question_id] = state
        self.mapping[utterance_id] = state.question_id
        # Keep capture metadata bounded across a long session.
        while len(self.groups) > 16:
            old_id, old_state = self.groups.popitem(last=False)
            for old_utterance in old_state.fragments:
                self.mapping.pop(old_utterance, None)
                self.activity.pop(old_utterance, None)
                self.partial_stamps.pop(old_utterance, None)
        return state

    def ingest(self, utterance_id, text, speech_at, *, final):
        if utterance_id in self.closed_utterances:
            return None
        if not final:
            if speech_at <= self.partial_stamps.get(utterance_id, float("-inf")):
                return None
            self.partial_stamps[utterance_id] = speech_at
        state = self._state_for(utterance_id, text, speech_at)
        if state.fragments.get(utterance_id) != text:
            state.fragments[utterance_id] = text
            state.revision += 1
        state.speech_at = max(state.speech_at, speech_at)
        if final:
            state.open_utterances.discard(utterance_id)
            self.closed_utterances.append(utterance_id)
            self.partial_stamps.pop(utterance_id, None)
        else:
            state.open_utterances.add(utterance_id)
        return AnswerRequest(state.question_id, state.text[:4000], state.speech_at,
                             not final, state.revision, tuple(state.fragments.values()))

    def complete_empty(self, utterance_id):
        """Close an empty or dropped utterance and discard its unverified partial."""
        if utterance_id in self.closed_utterances:
            return None
        self.closed_utterances.append(utterance_id)
        self.activity.pop(utterance_id, None)
        self.partial_stamps.pop(utterance_id, None)
        question_id = self.mapping.pop(utterance_id, None)
        if question_id is not None:
            state = self.groups[question_id]
            state.open_utterances.discard(utterance_id)
            state.fragments.pop(utterance_id, None)
            state.revision += 1
            if not state.fragments:
                del self.groups[question_id]
        return question_id

    def mark_semantic_status(self, question_id, incomplete):
        state = self.groups.get(question_id)
        if state:
            state.semantically_incomplete = incomplete

    def compact_committed(self, question_id, revision, normalized):
        state = self.groups.get(question_id)
        if state is None or state.revision != revision or state.open_utterances:
            return
        # Retain the model's checked question as the next continuation's base.
        # Never compact away wording captured during an in-flight older answer.
        keep = next(reversed(state.fragments))
        for utterance_id in list(state.fragments):
            if utterance_id != keep:
                self.mapping.pop(utterance_id, None)
                self.activity.pop(utterance_id, None)
        state.fragments = OrderedDict([(keep, normalized)])

    def can_submit(self, question_id, revision, now):
        state = self.groups.get(question_id)
        if state is None or state.revision != revision or state.open_utterances:
            return False
        if state is next(reversed(self.groups.values())):
            # Audio resumed but its first partial transcript has not arrived yet.
            if any(first - state.speech_at <= 8 for utterance_id, (first, _) in self.activity.items()
                   if utterance_id not in self.mapping):
                return False
        delay = max(self.pause_seconds, 2.2) if self.unfinished(state.text) or state.semantically_incomplete else self.pause_seconds
        return now - state.speech_at >= delay

    def poll_ready(self, now):
        ready = []
        for state in self.groups.values():
            if state.revision > state.submitted_revision and self.can_submit(state.question_id, state.revision, now):
                state.submitted_revision = state.revision
                ready.append(AnswerRequest(state.question_id, state.text[:4000], state.speech_at,
                                           False, state.revision, tuple(state.fragments.values())))
        return ready


class SemanticResponseDecoder:
    """Withhold final answer text until a valid semantic decision header arrives."""
    def __init__(self, on_text, on_metadata):
        self.on_text = on_text
        self.on_metadata = on_metadata
        self.buffer = ""
        self.metadata = None
        self.answer = []

    def feed(self, text):
        if self.metadata is not None:
            self.answer.append(text)
            self.on_text(text)
            return
        self.buffer += text
        if len(self.buffer) > 8000:
            raise ValueError("Model did not return the question decision header. Retry the question.")
        if "\n" not in self.buffer:
            return
        line, rest = self.buffer.split("\n", 1)
        try:
            metadata = json.loads(line.strip())
        except ValueError as exc:
            raise ValueError("Model did not return a valid question decision. No final answer was committed.") from exc
        if (not isinstance(metadata, dict) or metadata.get("relation") not in ("new", "continue", "incomplete")
                or not isinstance(metadata.get("question"), str) or len(metadata["question"]) > 4000):
            raise ValueError("Model returned an invalid question decision. No final answer was committed.")
        if metadata["relation"] != "incomplete" and not metadata["question"].strip():
            raise ValueError("Model did not identify the question. No final answer was committed.")
        self.metadata = metadata
        self.buffer = ""
        self.on_metadata(metadata)
        if rest:
            self.answer.append(rest)
            self.on_text(rest)

    def finish(self):
        if self.metadata is None:
            raise ValueError("Model did not finish its question decision header. Retry the question.")
        return "".join(self.answer).strip()


def commit_checked_answer(history, question, answer, *, metadata=None, previous=None,
                          draft=False, obsolete=False):
    """Only current, complete answers enter history; continuations revise one pair."""
    if draft or obsolete or (metadata and metadata["relation"] == "incomplete"):
        return None
    normalized = metadata["question"] if metadata else question
    continuing = metadata and metadata["relation"] == "continue"
    if (continuing and previous and previous["slot"] + 1 < len(history)
            and history[previous["slot"]]["content"] == previous["question"]):
        slot = previous["slot"]
        history[slot:slot + 2] = [{"role": "user", "content": normalized},
                                 {"role": "assistant", "content": answer}]
    else:
        slot = len(history)
        history.extend([{"role": "user", "content": normalized},
                        {"role": "assistant", "content": answer}])
    return {"question": normalized, "slot": slot}
