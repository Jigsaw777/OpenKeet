"""Offline regression checks for live drafts, question continuity and capture flags."""
import json
from pathlib import Path
import queue
import sys
import threading
import time
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from audio import SpeechListener
from core import build_messages, CopilotError
from live import (AnswerRequest, AudioChunk, LiveAnswerPlanner, QuestionBuffer,
                  SemanticResponseDecoder, TranscriptionMailbox, commit_checked_answer)


class MailboxTests(unittest.TestCase):
    def test_coalesces_partial_snapshots(self):
        box = TranscriptionMailbox()
        box.put(AudioChunk(1, "old audio", 1.0, False))
        box.put(AudioChunk(1, "new audio", 2.0, False))
        self.assertEqual(box.get().samples, "new audio")

    def test_final_has_priority_and_removes_its_partial(self):
        box = TranscriptionMailbox()
        box.put(AudioChunk(1, "partial", 1.0, False))
        box.put(AudioChunk(1, "complete", 2.0, True))
        self.assertEqual(box.get().samples, "complete")
        box.close()
        self.assertIsNone(box.get())

    def test_late_partial_after_final_is_ignored(self):
        box = TranscriptionMailbox()
        box.put(AudioChunk(1, "complete", 2.0, True))
        box.put(AudioChunk(1, "stale", 1.0, False))
        self.assertTrue(box.get().final)
        box.close()
        self.assertIsNone(box.get())

    def test_close_wakes_waiting_worker(self):
        box, values = TranscriptionMailbox(), []
        started = threading.Event()
        def work():
            started.set()
            values.append(box.get())
        thread = threading.Thread(target=work)
        thread.start()
        self.assertTrue(started.wait(1))
        box.close()
        thread.join(1)
        self.assertFalse(thread.is_alive())
        self.assertEqual(values, [None])

    def test_final_queue_is_bounded_and_reports_drop(self):
        box = TranscriptionMailbox(max_finals=2)
        self.assertIsNone(box.put(AudioChunk(1, "one", 1, True)))
        self.assertIsNone(box.put(AudioChunk(2, "two", 2, True)))
        self.assertEqual(box.put(AudioChunk(3, "three", 3, True)), 1)
        self.assertEqual(box.get().utterance_id, 2)

    def test_overflow_invalidates_partial_and_later_question_can_submit(self):
        mailbox = TranscriptionMailbox(max_finals=1)
        questions = QuestionBuffer(pause_seconds=1.25)
        mailbox.put(AudioChunk(0, "Explain the old topic", 0.0, False))
        partial = mailbox.get()
        questions.ingest(partial.utterance_id, partial.samples, partial.speech_at, final=False)
        mailbox.put(AudioChunk(0, "Explain the old topic in depth", 0.25, True))
        questions.observe_activity(1, 0.75)
        dropped = mailbox.put(AudioChunk(1, "How do promises work?", 1.0, True))
        self.assertEqual(dropped, 0)
        questions.complete_empty(dropped)
        self.assertIsNone(questions.ingest(0, "Delayed partial", 0.2, final=False))
        self.assertIn(1, questions.activity)
        retained = mailbox.get()
        questions.ingest(retained.utterance_id, retained.samples, retained.speech_at, final=True)
        self.assertEqual(questions.poll_ready(2.249), [])
        self.assertEqual([r.text for r in questions.poll_ready(2.25)], ["How do promises work?"])
        self.assertEqual(questions.poll_ready(60.0), [])


class PlannerTests(unittest.TestCase):
    def test_draft_requires_a_question_or_interview_prompt(self):
        planner = LiveAnswerPlanner()
        planner.offer_partial(1, "Thanks for joining us on the call this morning", 1)
        self.assertIsNone(planner.next_request())
        planner.offer_partial(2, "Okay, could you explain dependency injection", 2)
        self.assertTrue(planner.next_request().draft)

    def test_only_one_early_draft_per_question(self):
        planner = LiveAnswerPlanner()
        planner.offer_partial(1, "Explain Kotlin coroutine cancellation", 1, revision=1)
        self.assertTrue(planner.next_request().draft)
        planner.offer_partial(1, "Explain Kotlin coroutine cancellation with cleanup", 2, revision=2)
        self.assertIsNone(planner.next_request())

    def test_final_replaces_queued_partial(self):
        planner = LiveAnswerPlanner()
        planner.offer_partial(1, "Explain Kotlin coroutine cancellation", 1, revision=1)
        planner.offer_final(1, "Explain Kotlin coroutine cancellation with cleanup", 2, revision=2)
        request = planner.next_request()
        self.assertFalse(request.draft)
        self.assertIn("cleanup", request.text)
        self.assertIsNone(planner.next_request())

    def test_new_revision_replaces_older_queued_final(self):
        planner = LiveAnswerPlanner()
        planner.offer_final(1, "original question", 1, revision=1)
        planner.offer_final(1, "question with an added constraint", 2, revision=2)
        self.assertEqual(planner.next_request().revision, 2)
        self.assertIsNone(planner.next_request())

    def test_duplicate_and_stale_finals_are_not_resent(self):
        planner = LiveAnswerPlanner()
        planner.offer_final(1, "latest", 2, revision=2)
        planner.next_request()
        planner.offer_final(1, "latest", 2, revision=2)
        planner.offer_final(1, "older", 1, revision=1)
        self.assertIsNone(planner.next_request())

    def test_waiting_draft_retries_only_after_more_question_content(self):
        planner = LiveAnswerPlanner()
        planner.offer_partial(1, "Can you tell me a little bit about", 1, revision=1)
        first = planner.next_request()
        planner.note_draft_result(first, "Listening for the question…")
        planner.offer_partial(1, "Can you tell me a little bit about Kotlin", 2, revision=2)
        self.assertIsNone(planner.next_request())
        planner.offer_partial(1, "Can you tell me a little bit about Kotlin coroutine cancellation and resource cleanup", 3, revision=3)
        self.assertTrue(planner.next_request().draft)

    def test_waiting_drafts_do_not_create_unbounded_requests(self):
        planner = LiveAnswerPlanner()
        planner.offer_partial(1, "Can you tell me a little bit about", 1)
        first = planner.next_request()
        planner.note_draft_result(first, "Listening for the question…")
        planner.offer_partial(1, first.text + " Kotlin coroutine cancellation and cleanup", 2)
        second = planner.next_request()
        planner.note_draft_result(second, "Listening for the question…")
        planner.offer_partial(1, second.text + " and many further words about the same topic", 3)
        self.assertIsNone(planner.next_request())


class QuestionBufferTests(unittest.TestCase):
    def test_later_empty_utterance_does_not_release_earlier_pending_speech(self):
        buffer = QuestionBuffer(1.25)
        first = buffer.ingest(0, "Explain optimistic locking", 0.0, final=True)
        buffer.observe_activity(1, 0.5)
        buffer.observe_activity(2, 1.0)
        buffer.complete_empty(2)
        self.assertFalse(buffer.can_submit(first.utterance_id, first.revision, 1.5))
        buffer.complete_empty(1)
        self.assertTrue(buffer.can_submit(first.utterance_id, first.revision, 1.5))

    def test_duplicate_and_out_of_order_transcripts_do_not_reopen_question(self):
        buffer = QuestionBuffer(1.25)
        partial = buffer.ingest(0, "How does structured concurrency work", 2.0, final=False)
        self.assertIsNone(buffer.ingest(0, "How does structured", 1.75, final=False))
        self.assertIsNone(buffer.ingest(0, partial.text, 2.0, final=False))
        final = buffer.ingest(0, partial.text + " in Kotlin?", 2.25, final=True)
        self.assertIsNone(buffer.ingest(0, "Different duplicate final", 2.5, final=True))
        buffer.observe_activity(0, 3.0)
        self.assertIsNone(buffer.ingest(0, partial.text, 2.0, final=False))
        self.assertEqual(buffer.poll_ready(3.499), [])
        self.assertEqual([r.text for r in buffer.poll_ready(3.5)], [final.text])

    def test_rolling_transcript_replaces_same_fragment(self):
        buffer = QuestionBuffer()
        buffer.observe_activity(0, 0)
        first = buffer.ingest(0, "Explain Kotlin coroutines", 0.9, final=False)
        second = buffer.ingest(0, "Explain Kotlin coroutine cancellation", 1.8, final=False)
        self.assertEqual(first.utterance_id, second.utterance_id)
        self.assertEqual(second.text, "Explain Kotlin coroutine cancellation")
        self.assertEqual(len(second.segments), 1)
        self.assertGreater(second.revision, first.revision)

    def test_short_pause_does_not_finalize_before_debounce(self):
        buffer = QuestionBuffer(pause_seconds=1.2)
        buffer.ingest(0, "Explain structured concurrency", 1.0, final=True)
        self.assertEqual(buffer.poll_ready(2.1), [])
        self.assertEqual(len(buffer.poll_ready(2.3)), 1)
        self.assertEqual(buffer.poll_ready(3.0), [])

    def test_resume_before_new_transcript_blocks_old_final(self):
        buffer = QuestionBuffer()
        request = buffer.ingest(0, "Explain coroutine cancellation", 1.0, final=True)
        buffer.observe_activity(1, 1.9)
        self.assertFalse(buffer.can_submit(request.utterance_id, request.revision, 3.0))
        continued = buffer.ingest(1, "and how resources are cleaned up", 2.5, final=False)
        self.assertEqual(request.utterance_id, continued.utterance_id)
        self.assertIn("resources", continued.text)

    def test_continuation_with_added_constraint_is_same_question(self):
        buffer = QuestionBuffer()
        first = buffer.ingest(0, "How would you protect shared state", 1.0, final=True)
        buffer.observe_activity(1, 3.0)
        second = buffer.ingest(1, "but you cannot block a thread", 4.0, final=True)
        self.assertEqual(first.utterance_id, second.utterance_id)
        self.assertIn("cannot block", second.text)
        self.assertEqual(len(second.segments), 2)
        self.assertEqual(buffer.poll_ready(5.0), [])
        final = buffer.poll_ready(5.3)[0]
        self.assertEqual(final.text, "How would you protect shared state but you cannot block a thread")

    def test_explicit_next_question_starts_new_group(self):
        buffer = QuestionBuffer()
        first = buffer.ingest(0, "What is Kotlin", 1.0, final=True)
        buffer.observe_activity(1, 1.6)
        second = buffer.ingest(1, "Next question, explain database indexes", 2.1, final=True)
        self.assertNotEqual(first.utterance_id, second.utterance_id)

    def test_long_gap_and_distinct_topic_start_new_group(self):
        buffer = QuestionBuffer()
        first = buffer.ingest(0, "Explain coroutine cancellation", 1.0, final=True)
        buffer.observe_activity(1, 11.0)
        second = buffer.ingest(1, "Describe database transaction isolation", 12.0, final=True)
        self.assertNotEqual(first.utterance_id, second.utterance_id)

    def test_open_speech_never_commits_based_only_on_time(self):
        buffer = QuestionBuffer()
        buffer.ingest(0, "Explain how cancellation propagates", 1.0, final=False)
        self.assertEqual(buffer.poll_ready(100.0), [])

    def test_unfinished_clause_waits_longer(self):
        buffer = QuestionBuffer()
        buffer.ingest(0, "Explain coroutine cancellation when", 1.0, final=True)
        self.assertEqual(buffer.poll_ready(2.5), [])
        self.assertEqual(len(buffer.poll_ready(3.3)), 1)

    def test_short_complete_question_is_not_forced_to_wait_extra(self):
        buffer = QuestionBuffer()
        buffer.ingest(0, "What is Kotlin", 1.0, final=True)
        self.assertFalse(buffer.unfinished("What is Kotlin"))
        self.assertEqual(len(buffer.poll_ready(2.3)), 1)

    def test_noise_does_not_block_question_forever(self):
        buffer = QuestionBuffer()
        first = buffer.ingest(0, "Explain coroutine cancellation", 1.0, final=True)
        buffer.observe_activity(1, 1.8)
        self.assertEqual(buffer.poll_ready(3.0), [])
        buffer.complete_empty(1)
        self.assertTrue(buffer.can_submit(first.utterance_id, first.revision, 3.0))

    def test_model_incomplete_status_preserves_continuation(self):
        buffer = QuestionBuffer()
        first = buffer.ingest(0, "Can you explain the behavior", 1.0, final=True)
        buffer.mark_semantic_status(first.utterance_id, True)
        buffer.observe_activity(1, 10.0)
        second = buffer.ingest(1, "of cancellation in a supervisor scope", 11.0, final=True)
        self.assertEqual(first.utterance_id, second.utterance_id)
        self.assertIn("supervisor", second.text)

    def test_old_revision_cannot_be_submitted_after_text_changes(self):
        buffer = QuestionBuffer()
        old = buffer.ingest(0, "Explain cancellation", 1.0, final=False)
        buffer.ingest(0, "Explain cancellation with a supervisor job", 2.0, final=True)
        self.assertFalse(buffer.can_submit(old.utterance_id, old.revision, 5.0))

    def test_committed_question_becomes_compact_continuation_base(self):
        buffer = QuestionBuffer()
        buffer.ingest(0, "Explain shared state", 1.0, final=True)
        buffer.observe_activity(1, 2.0)
        ready = buffer.ingest(1, "but without blocking a thread", 3.0, final=True)
        normalized = "How do you protect shared state without blocking a thread?"
        buffer.compact_committed(ready.utterance_id, ready.revision, normalized)
        buffer.observe_activity(2, 5.0)
        revised = buffer.ingest(2, "and support cancellation", 6.0, final=True)
        self.assertEqual(revised.text, normalized + " and support cancellation")
        self.assertEqual(len(revised.segments), 2)

    def test_old_answer_cannot_compact_away_new_constraints(self):
        buffer = QuestionBuffer()
        old = buffer.ingest(0, "Explain shared state", 1.0, final=True)
        buffer.observe_activity(1, 2.0)
        revised = buffer.ingest(1, "but without blocking", 3.0, final=True)
        buffer.compact_committed(old.utterance_id, old.revision, "Explain shared state")
        self.assertIn("without blocking", buffer.groups[revised.utterance_id].text)


class SemanticCheckTests(unittest.TestCase):
    def test_final_prompt_has_segments_and_previous_question(self):
        messages = build_messages({}, "Concise answer", "protect state without blocking", speech_context={
            "segments": ["protect shared state", "but do not block"], "previous_question": "protect shared state"})
        self.assertIn("SPOKEN QUESTION CHECK", messages[0]["content"])
        value = json.loads(messages[-1]["content"])
        self.assertEqual(value["speech_segments"][-1], "but do not block")
        self.assertEqual(value["previous_completed_question"], "protect shared state")

    def test_draft_does_not_invent_unheard_constraints(self):
        messages = build_messages({}, "Concise answer", "Explain cancellation", draft=True)
        self.assertIn("Do not assume the rest of the question", messages[0]["content"])

    def test_split_metadata_is_hidden_until_valid_then_answer_streams(self):
        text, metadata = [], []
        decoder = SemanticResponseDecoder(text.append, metadata.append)
        decoder.feed('{"relation":"con')
        self.assertEqual(text, [])
        decoder.feed('tinue","question":"How can this avoid blocking?"}\nUse ')
        decoder.feed("a suspending Mutex.")
        self.assertEqual(decoder.finish(), "Use a suspending Mutex.")
        self.assertEqual(metadata[0]["relation"], "continue")
        self.assertNotIn("relation", "".join(text))

    def test_invalid_semantic_header_never_commits_answer_text(self):
        texts = []
        decoder = SemanticResponseDecoder(texts.append, lambda _: None)
        with self.assertRaises(ValueError):
            decoder.feed("I assume the question is complete.\nUse a lock.")
        self.assertEqual(texts, [])

    def test_incomplete_and_obsolete_answers_stay_out_of_history(self):
        history = []
        self.assertIsNone(commit_checked_answer(history, "q", "draft", draft=True))
        self.assertIsNone(commit_checked_answer(history, "q", "old", obsolete=True))
        self.assertIsNone(commit_checked_answer(history, "q", "wait", metadata={"relation": "incomplete", "question": "q"}))
        self.assertEqual(history, [])

    def test_semantic_continuation_replaces_prior_pair(self):
        history = []
        previous = commit_checked_answer(history, "How to protect shared state?", "Use a lock.")
        current = commit_checked_answer(history, "but no blocking", "Use a suspending Mutex.",
                    metadata={"relation": "continue", "question": "How to protect shared state without blocking?"}, previous=previous)
        self.assertEqual(len(history), 2)
        self.assertIn("without blocking", history[0]["content"])
        self.assertNotIn("Use a lock.", history[1]["content"])
        self.assertEqual(current["slot"], 0)

    def test_distinct_question_creates_new_pair(self):
        history = []
        previous = commit_checked_answer(history, "Kotlin?", "A language.")
        commit_checked_answer(history, "Indexes?", "A data structure.",
                              metadata={"relation": "new", "question": "What is an index?"}, previous=previous)
        self.assertEqual(len(history), 4)


class LiveAudioTests(unittest.TestCase):
    def test_partial_transcript_is_emitted_before_speaker_finishes(self):
        try:
            import numpy as np
        except ImportError:
            self.skipTest("Install numpy for the audio pipeline check")
        partial_seen = threading.Event()
        partial_before_stop, emitted = [], []
        def emit(kind, data):
            emitted.append((kind, data))
            if kind == "partial_transcript":
                partial_before_stop.append(not listener.stop_event.is_set())
                partial_seen.set()
        listener = SpeechListener(emit)
        class Model:
            def transcribe(self, samples, **kwargs):
                text = "Explain Kotlin coroutine cancellation" if len(samples) < 30000 else "Explain Kotlin coroutine cancellation with resource cleanup"
                return [SimpleNamespace(text=text)], None
        listener.model = Model()
        class Recorder:
            count = 0
            def __enter__(self):
                return self
            def __exit__(self, *args):
                pass
            def record(self, numframes):
                self.count += 1
                if self.count == 35 and not partial_seen.wait(2):
                    raise RuntimeError("No partial recognition occurred while speech continued")
                if self.count <= 65:
                    return np.full((480, 2), 0.1, dtype="float32")
                if self.count > 82:
                    listener.stop()
                return np.zeros((480, 2), dtype="float32")
        mic = SimpleNamespace(id="fake", isloopback=True, name="Fake output", recorder=lambda **kwargs: Recorder())
        fake = SimpleNamespace(default_speaker=lambda: SimpleNamespace(id="fake"),
                               get_microphone=lambda **kwargs: mic)
        with patch("audio.sys.platform", "linux"), patch.dict(sys.modules, {"soundcard": fake}):
            listener.start(("system", None))
            for thread in listener.threads:
                thread.join(3)
        self.assertTrue(partial_seen.is_set())
        self.assertTrue(any(partial_before_stop))
        finals = [data["text"] for kind, data in emitted if kind == "transcript"]
        self.assertEqual(finals, ["Explain Kotlin coroutine cancellation with resource cleanup"])
        self.assertFalse(any(t.is_alive() for t in listener.threads))

    def test_final_revision_cancels_obsolete_live_draft_in_controller(self):
        from app import App
        class Scalar:
            def __init__(self, value=None):
                self.value = value
            def get(self):
                return self.value
            def set(self, value):
                self.value = value
        app = App.__new__(App)
        app.auto, app.audio_active, app.busy = Scalar(True), True, True
        app.question_pause = Scalar(1.2)
        app.questions, app.audio_answers = QuestionBuffer(), LiveAnswerPlanner()
        stamp = time.monotonic() - 2
        ready = app.questions.ingest(0, "Explain cancellation without blocking", stamp, final=True)
        app.current_audio_request = AnswerRequest(ready.utterance_id, "Explain cancellation", stamp, True, 0)
        app.current_draft, app.superseded, app.auto_timer = True, False, None
        app.cancel = threading.Event()
        app.answer_heading, app.audio_status = Scalar(), Scalar()
        app.poll_audio_answers()
        self.assertTrue(app.superseded)
        self.assertTrue(app.cancel.is_set())
        self.assertTrue(app.audio_answers.pending)


class ControllerEventTests(unittest.TestCase):
    def setUp(self):
        from app import App
        self.app = app = App.__new__(App)
        app.root, app.answer, app.status, app.answer_heading = Mock(), Mock(), Mock(), Mock()
        app.generate_button, app.stop_button = Mock(), Mock()
        app.live_question, app.audio_status = Mock(), Mock()
        app._append, app._replace = Mock(), Mock()
        app.questions, app.audio_answers = QuestionBuffer(1.25), LiveAnswerPlanner()
        app.events, app.history = queue.Queue(), []
        app.auto, app.live_drafts = Mock(), Mock()
        app.auto.get.return_value = False
        app.audio_active = True
        app.busy, app.cancel = True, threading.Event()
        app.current_audio_request = None
        app.current_obsolete = app.current_draft = app.superseded = False
        app.current_semantic = app.previous_final_snapshot = None
        app.current_question, app.current_answer = "How do promises work?", ""
        app.current_speech_end, app.first_token_seconds = None, None
        app.started, app.latency_note = 1.0, ""

    def test_overflow_clears_provisional_answer_and_ignores_late_tokens(self):
        app = self.app
        app.current_audio_request = app.questions.ingest(0, "Explain the old topic", 0.0, final=False)
        app.events.put(("speech_dropped", 0))
        app.events.put(("token", (app.cancel, "late draft text")))
        app.events.put(("done", (app.cancel, ("late draft text", None))))
        app.pump()
        app._append.assert_not_called()
        self.assertEqual(app.history, [])
        self.assertEqual(app.questions.groups, {})
        self.assertTrue(app.cancel.is_set())

    def test_cancelled_request_cannot_commit_even_if_model_queued_success(self):
        app = self.app
        app.events.put(("done", (app.cancel, ("Completed before the UI processed it", None))))
        app.cancel.set()
        app.pump()
        self.assertEqual(app.history, [])
        self.assertFalse(app.busy)

    def test_provider_error_discards_partial_output_and_keeps_error_visible(self):
        app = self.app
        app.events.put(("token", (threading.Event(), "Token from a previous generation")))
        app.events.put(("done", (app.cancel, ("", "Provider connection failed"))))
        app.pump()
        app._append.assert_not_called()
        app._replace.assert_called_once_with(app.answer, "")
        app.status.set.assert_called_once_with("Provider connection failed")
        self.assertEqual(app.history, [])

    def test_resumed_speech_defers_an_inflight_checked_answer(self):
        app = self.app
        app.questions.ingest(0, app.current_question, 0.0, final=True)
        app.current_audio_request, = app.questions.poll_ready(1.25)
        app.current_semantic = {"relation": "new", "question": app.current_question}
        app.events.put(("speech_activity", {"utterance_id": 1, "at": 1.5}))
        app.events.put(("done", (app.cancel, ("Old answer", None))))
        with patch("app.time.monotonic", return_value=1.75):
            app.pump()
        self.assertEqual(app.history, [])
        self.assertTrue(app.current_obsolete)
        self.assertEqual(app.audio_answers.next_request(), app.current_audio_request)


class CaptureFlagTests(unittest.TestCase):
    def test_windows_exclusion_sets_and_reads_back_native_flag(self):
        import ctypes
        from capture_privacy import set_capture_exclusion
        value = {"flag": 0}
        class Function:
            def __init__(self, fn):
                self.fn = fn
            def __call__(self, *args):
                return self.fn(*args)
        def setter(handle, flag):
            value["flag"] = flag
            return 1
        def getter(handle, pointer):
            pointer._obj.value = value["flag"]
            return 1
        fake = SimpleNamespace(GetAncestor=Function(lambda *args: 123),
                               SetWindowDisplayAffinity=Function(setter),
                               GetWindowDisplayAffinity=Function(getter))
        root = SimpleNamespace(update_idletasks=lambda: None, winfo_id=lambda: 42)
        with patch("capture_privacy.sys.platform", "win32"), \
             patch.object(sys, "getwindowsversion", return_value=SimpleNamespace(build=19045), create=True), \
             patch.object(ctypes, "WinDLL", return_value=fake, create=True):
            self.assertIn("enabled", set_capture_exclusion(root, True))
            self.assertEqual(value["flag"], 0x11)
            self.assertIn("disabled", set_capture_exclusion(root, False))
            self.assertEqual(value["flag"], 0)

    def test_readback_mismatch_is_reported_as_failure(self):
        import ctypes
        from capture_privacy import set_capture_exclusion
        class Function:
            def __init__(self, fn):
                self.fn = fn
            def __call__(self, *args):
                return self.fn(*args)
        fake = SimpleNamespace(GetAncestor=Function(lambda *args: 123),
                               SetWindowDisplayAffinity=Function(lambda *args: 1),
                               GetWindowDisplayAffinity=Function(lambda *args: 1))
        root = SimpleNamespace(update_idletasks=lambda: None, winfo_id=lambda: 42)
        with patch("capture_privacy.sys.platform", "win32"), \
             patch.object(sys, "getwindowsversion", return_value=SimpleNamespace(build=19045), create=True), \
             patch.object(ctypes, "WinDLL", return_value=fake, create=True):
            with self.assertRaises(CopilotError):
                set_capture_exclusion(root, True)


if __name__ == "__main__":
    unittest.main()
