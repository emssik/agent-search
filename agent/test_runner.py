"""Tests confirming bugs in runner.py (and verifying fixes)."""
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

# Make runner importable without google-genai installed
sys.modules.setdefault("dotenv", MagicMock(load_dotenv=lambda: None))
sys.modules.setdefault("google", MagicMock())
sys.modules.setdefault("google.genai", MagicMock())
sys.modules.setdefault("google.genai.types", MagicMock())

import agent.runner as runner_module


# ═══════════════════════════════════════════════════════════════════════════════
# BUG 1 (HIGH): read_file path traversal
# ═══════════════════════════════════════════════════════════════════════════════


class TestReadFilePathTraversal(unittest.TestCase):
    """read_file must refuse paths that resolve outside corpus."""

    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.corpus = Path(self._tmpdir.name)
        (self.corpus / "note.txt").write_text("hello from corpus\n")
        # Create a sensitive file outside the corpus
        self.secret = self.corpus.parent / "secret.txt"
        self.secret.write_text("top-secret\n")
        tools = runner_module._build_tools(str(self.corpus))
        self.read_file = next(f for f in tools if f.__name__ == "read_file")

    def tearDown(self):
        self._tmpdir.cleanup()
        self.secret.unlink(missing_ok=True)

    def test_relative_traversal_is_blocked(self):
        """../../secret.txt must NOT be readable."""
        with self.assertRaises(PermissionError):
            self.read_file("../../secret.txt")

    def test_absolute_outside_corpus_is_blocked(self):
        """Absolute path outside corpus must NOT be readable."""
        with self.assertRaises(PermissionError):
            self.read_file(str(self.secret))

    def test_safe_relative_path_works(self):
        """Normal relative path inside corpus must still work."""
        result = self.read_file("note.txt")
        self.assertIn("hello from corpus", result)

    def test_safe_absolute_path_inside_corpus_works(self):
        """Absolute path inside corpus must work."""
        result = self.read_file(str(self.corpus / "note.txt"))
        self.assertIn("hello from corpus", result)


# ═══════════════════════════════════════════════════════════════════════════════
# BUG 2 (MEDIUM): agent loop only handles parts[0] / single function_call
# ═══════════════════════════════════════════════════════════════════════════════


class TestAgentLoopMultipleParts(unittest.TestCase):
    """run_agent must process all parts returned by the model, not just parts[0]."""

    def _make_fc_part(self, name, args):
        fc = MagicMock()
        fc.function_call = MagicMock()
        fc.function_call.name = name
        fc.function_call.args = args
        return fc

    def test_two_function_calls_both_executed(self):
        """If model returns two function_call parts, both must be dispatched."""
        with tempfile.TemporaryDirectory() as corpus_dir:
            call_log = []

            # Patch _build_tools to inject a spy tool
            original_build = runner_module._build_tools

            def patched_build(c):
                tools = original_build(c)

                def spy_tool(x: str = "") -> str:
                    call_log.append(x)
                    return "ok"

                return tools + [spy_tool]

            fc1 = MagicMock()
            fc1.function_call = MagicMock(name_attr="spy_tool")
            fc1.function_call.name = "spy_tool"
            fc1.function_call.args = {"x": "first"}

            fc2 = MagicMock()
            fc2.function_call = MagicMock()
            fc2.function_call.name = "spy_tool"
            fc2.function_call.args = {"x": "second"}

            # Turn 1: two function_call parts
            content_t1 = MagicMock()
            content_t1.parts = [fc1, fc2]

            # Turn 2: final text answer
            text_part = MagicMock()
            text_part.function_call = None
            content_t2 = MagicMock()
            content_t2.parts = [text_part]

            resp1 = MagicMock()
            resp1.candidates = [MagicMock(content=content_t1)]
            resp2 = MagicMock()
            resp2.candidates = [MagicMock(content=content_t2)]
            resp2.text = "done"

            mock_client = MagicMock()
            mock_client.models.generate_content.side_effect = [resp1, resp2]

            with patch.object(runner_module, "_build_tools", patched_build), \
                 patch("agent.runner.genai") as mock_genai:
                mock_genai.Client.return_value = mock_client
                runner_module.run_agent(
                    task="test", corpus=corpus_dir, max_turns=5
                )

            self.assertIn("first", call_log, "first function_call was not dispatched")
            self.assertIn("second", call_log, "second function_call was not dispatched")


# ═══════════════════════════════════════════════════════════════════════════════
# BUG 3 (MEDIUM): agent-search errors treated as valid results
# ═══════════════════════════════════════════════════════════════════════════════


class TestAgentSearchErrorPropagation(unittest.TestCase):
    """_run_agent_search must signal failure when returncode != 0."""

    def test_returncode_returned(self):
        """_run_agent_search must return the real returncode."""
        with patch("agent.runner.subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                stdout="error: bad regex\n",
                stderr="",
                returncode=1,
            )
            output, code = runner_module._run_agent_search(["search", "-q", "x"], "/corpus")
        self.assertEqual(code, 1, "returncode 1 must be passed through")

    def test_search_tool_raises_on_nonzero(self):
        """search() tool must raise RuntimeError when CLI exits with error."""
        with tempfile.TemporaryDirectory() as corpus_dir:
            tools = runner_module._build_tools(corpus_dir)
            search_fn = next(f for f in tools if f.__name__ == "search")

            with patch("agent.runner._run_agent_search", return_value=("error output", 1)):
                with self.assertRaises(RuntimeError):
                    search_fn(query=["test"])

    def test_grep_tool_raises_on_nonzero(self):
        """grep() tool must raise RuntimeError when CLI exits with error."""
        with tempfile.TemporaryDirectory() as corpus_dir:
            tools = runner_module._build_tools(corpus_dir)
            grep_fn = next(f for f in tools if f.__name__ == "grep")

            with patch("agent.runner._run_agent_search", return_value=("error output", 2)):
                with self.assertRaises(RuntimeError):
                    grep_fn(pattern="test")


# ═══════════════════════════════════════════════════════════════════════════════
# BUG 4 (MEDIUM): glob() searches CWD instead of corpus
# ═══════════════════════════════════════════════════════════════════════════════


class TestGlobSearchesCorpus(unittest.TestCase):
    """glob() must search inside corpus, not the current working directory."""

    def test_glob_finds_corpus_file(self):
        with tempfile.TemporaryDirectory() as corpus_dir:
            corpus = Path(corpus_dir)
            (corpus / "note.txt").write_text("hello from corpus\n")
            (corpus / "subdir").mkdir()
            (corpus / "subdir" / "deep.txt").write_text("deep")

            tools = runner_module._build_tools(corpus_dir)
            glob_fn = next(f for f in tools if f.__name__ == "glob")

            result = glob_fn("**/*.txt")
            self.assertIn("note.txt", result, "corpus file must appear in glob results")
            self.assertIn("deep.txt", result, "nested corpus file must appear in glob results")

    def test_glob_does_not_leak_cwd(self):
        """Files from CWD that are not in corpus must NOT appear."""
        with tempfile.TemporaryDirectory() as corpus_dir:
            tools = runner_module._build_tools(corpus_dir)
            glob_fn = next(f for f in tools if f.__name__ == "glob")

            result = glob_fn("*.py")
            # runner.py lives in CWD/agent/, not in corpus — must not appear
            self.assertNotIn("runner.py", result)


# ─── entry point ──────────────────────────────────────────────────────────────

if __name__ == "__main__":
    unittest.main(verbosity=2)
