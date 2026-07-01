#!/usr/bin/env python3
"""
check_config.py — validate a LiteLLM proxy config.yaml against the on-disk
schema index corpus.

Read-only: this script never modifies the config. It encodes checks (a)-(i)
(source of truth, mirrored by the 8 constraint-litellm-* skills) and the
8 LiteLLM config constraints:
  config-schema / in-memory-no-db / secret-hygiene / provider-prefix /
  openai-compatible-api-base / anthropic-suffix / fallback-resolution /
  deprecation-free.

Checks:
  (a) every config key exists in config-yaml.option-index.json under the
      correct section (unknown keys and wrong-section keys are FAIL).
  (b) every litellm_params.model prefix is a valid provider prefix.
  (c) every router_settings.fallbacks source + target resolves to a
      model_name in model_list.
  (d) deprecated keys are flagged (WARN) with their replacement.
  (e) in-memory mode: requires_db=true keys are FAIL (except
      disable_spend_logs:true which is the intentional compensating control
      -> WARN); DB-requiring env vars (DATABASE_URL/STORE_MODEL_IN_DB/...) are
      FAIL.
  (f) requires_redis=true keys are FAIL in in-memory mode; REDIS_* env vars
      are FAIL.
  (g) every os.environ/<VAR> resolves to a known env var (WARN if unknown,
      FAIL if a clear misspelling of a built-in); master_key must resolve to
      os.environ/LITELLM_MASTER_KEY.
  (h) openai/ entries: api_base ends with /v1, no appended endpoint path,
      api_key present.
  (i) anthropic/ entries: api_base does not pre-include /v1/messages unless
      LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX is set.

Exit code: 0 on PASS (no FAIL), 1 on any FAIL. WARNs do not fail the gate.

Usage:
  check_config.py --config <path> --schemas-dir <dir> [--mode in-memory|db-backed]
  check_config.py --config <path> --schemas <dir>   # --schemas is an alias

Dependencies: Python 3 stdlib + PyYAML.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from typing import Any, Dict, List, Optional, Set, Tuple

try:
    import yaml  # type: ignore
except ImportError:  # pragma: no cover
    sys.stderr.write(
        "ERROR: PyYAML is required but not installed. "
        "Install with `pip install pyyaml` (or run inside `nix develop`).\n"
    )
    sys.exit(2)


# A finding is: (check_letter, severity, key_path, message)
Finding = Tuple[str, str, str, str]

# Sensitive keys whose values must use os.environ/ indirection (never literals).
# Grounded in the doc's "Safety/security notes": secrets must never appear in
# config.yaml. Used by check (g) for the master_key rule and as the secret-
# hygiene aspect of the config-check skill.
SENSITIVE_KEYS: Set[str] = {
    "master_key",
    "api_key",
    "password",
    "redis_password",
    "database_url",
    "salt_key",
}


def load_index(path: str, label: str) -> Any:
    """Load a JSON index file, exiting with a clear error if missing/invalid."""
    if not os.path.isfile(path):
        sys.stderr.write(f"ERROR: {label} index not found: {path}\n")
        sys.exit(2)
    try:
        with open(path, "r", encoding="utf-8") as fh:
            return json.load(fh)
    except (json.JSONDecodeError, OSError) as exc:
        sys.stderr.write(f"ERROR: failed to parse {label} index {path}: {exc}\n")
        sys.exit(2)


def build_option_lookup(
    options: List[Dict[str, Any]]
) -> Tuple[Dict[str, List[Dict[str, Any]]], Set[str]]:
    """Return (key -> [entries], set of all sections)."""
    by_key: Dict[str, List[Dict[str, Any]]] = {}
    sections: Set[str] = set()
    for entry in options:
        by_key.setdefault(entry["key"], []).append(entry)
        sections.add(entry["section"])
    return by_key, sections


def collect_valid_prefixes(providers: List[Dict[str, Any]]) -> Set[str]:
    """Collect valid litellm_prefix values from provider-fields.index.json.

    Handles prefixes stored as either a string or a list (e.g. Ollama has
    ['ollama/', 'ollama_chat/']). Also adds the prefixes the source-of-truth
    skill doc lists as known-valid but which are absent from the provider index
    (vllm/, text-completion-openai/, text-completion-inception/) so we never
    false-positive on a prefix the doc explicitly permits.
    """
    prefixes: Set[str] = set()
    for prov in providers:
        val = prov.get("litellm_prefix")
        if isinstance(val, list):
            prefixes.update(val)
        elif isinstance(val, str):
            prefixes.add(val)
    # Grounded in validate-litellm-config.md check (b) "Known valid prefixes".
    prefixes.update(
        {"vllm/", "text-completion-openai/", "text-completion-inception/"}
    )
    return prefixes


def collect_env_var_names(env_vars: List[Dict[str, Any]]) -> Set[str]:
    return {e["name"] for e in env_vars}


def env_var_requires_db(env_vars: List[Dict[str, Any]]) -> Set[str]:
    return {e["name"] for e in env_vars if e.get("requires_db") is True}


def env_var_deprecated(env_vars: List[Dict[str, Any]]) -> Dict[str, Optional[str]]:
    return {
        e["name"]: e.get("replacement")
        for e in env_vars
        if e.get("deprecated") is True
    }


ENV_REF_RE = re.compile(r"os\.environ/([A-Za-z_][A-Za-z0-9_]*)")


def extract_env_refs(obj: Any) -> List[Tuple[str, str]]:
    """Walk obj recursively; return [(var_name, dotted_path)] for every
    os.environ/<VAR> string value found."""
    out: List[Tuple[str, str]] = []

    def walk(node: Any, path: str) -> None:
        if isinstance(node, str):
            m = ENV_REF_RE.search(node)
            if m:
                out.append((m.group(1), path))
        elif isinstance(node, dict):
            for k, v in node.items():
                walk(v, f"{path}.{k}" if path else str(k))
        elif isinstance(node, list):
            for i, item in enumerate(node):
                walk(item, f"{path}[{i}]")

    walk(obj, "")
    return out


def load_yaml_mapping(path: str) -> Optional[Dict[str, Any]]:
    """Load a YAML file as a dict; return None if missing/unparseable/not a
    mapping. Used by include resolution and project-var collection."""
    if not os.path.isfile(path):
        return None
    try:
        with open(path, "r", encoding="utf-8") as fh:
            data = yaml.safe_load(fh)
    except (yaml.YAMLError, OSError):
        return None
    if not isinstance(data, dict):
        return None
    return data


def resolve_includes(config: Dict[str, Any], config_path: str) -> Dict[str, Any]:
    """Resolve a LiteLLM `include:` directive by merging child files.

    LiteLLM's `include:` (a str or list of paths) pulls in additional YAML
    files whose top-level sections are merged into the parent config. Child
    paths resolve RELATIVE TO THE CONFIG FILE'S DIRECTORY. PyYAML resolves
    anchors and `<<:` merge keys when each file is parsed, so anchors are
    file-local (an anchor in models.yaml is resolved within models.yaml
    before merging). List-valued sections (e.g. model_list) are concatenated
    (parent first, then child); dict-valued sections are shallow-merged (child
    overrides parent); other types are overridden by the child.

    Read-only: never writes to disk. Robust: a missing or unparseable child
    file is skipped with a stderr warning (does not abort). The `include` key
    itself is a valid top-level option (present in config-yaml.option-index.json)
    and is left in place for check (a).
    """
    includes = config.get("include")
    if includes is None:
        return config
    if isinstance(includes, str):
        includes = [includes]
    if not isinstance(includes, list):
        return config

    base_dir = os.path.dirname(os.path.abspath(config_path))
    merged: Dict[str, Any] = dict(config)
    for rel in includes:
        if not isinstance(rel, str) or not rel:
            continue
        child_path = rel if os.path.isabs(rel) else os.path.join(base_dir, rel)
        child = load_yaml_mapping(child_path)
        if child is None:
            sys.stderr.write(
                f"WARNING: include '{rel}' could not be loaded "
                f"(resolved: {child_path}); skipping\n"
            )
            continue
        for key, val in child.items():
            if (
                key in merged
                and isinstance(merged[key], list)
                and isinstance(val, list)
            ):
                merged[key] = merged[key] + val
            elif (
                key in merged
                and isinstance(merged[key], dict)
                and isinstance(val, dict)
            ):
                merged_key: Dict[str, Any] = dict(merged[key])
                merged_key.update(val)
                merged[key] = merged_key
            else:
                merged[key] = val
    return merged


def collect_project_defined_vars(infra_path: str) -> Set[str]:
    """Parse the canonical deployment config (following `include:`) and collect
    every os.environ/<VAR> it references. Per env-vars.index.json coverage_note,
    project-defined vars (KIMI_CODE_API_KEY, MINIMAX_CODING_API_KEY,
    NEURALWATT_API_KEY) are referenced in the config (now via models.yaml
    included by config.yaml). This makes check (g) recognise project-defined
    vars even if they are absent from the index."""
    data = load_yaml_mapping(infra_path)
    if data is None:
        return set()
    data = resolve_includes(data, infra_path)
    return {var for var, _ in extract_env_refs(data)}


def is_misspelling_of_known(var: str, known: Set[str]) -> Optional[str]:
    """Conservative misspelling detector grounded in the doc's example
    (OPENROUTER_KEY vs OPENROUTER_API_KEY). Returns the correct name if `var`
    is a clear misspelling of a known built-in, else None."""
    # Rule 1: <X>_KEY where <X>_API_KEY is a known built-in.
    if var.endswith("_KEY"):
        candidate = var[: -len("_KEY")] + "_API_KEY"
        if candidate in known:
            return candidate
    # Rule 2: <X>_APIKEY (missing underscore) where <X>_API_KEY is known.
    if var.endswith("_APIKEY"):
        candidate = var[: -len("_APIKEY")] + "_API_KEY"
        if candidate in known:
            return candidate
    return None


# ---------------------------------------------------------------------------
# Check (a) + (d) + (e) + (f): walk the config validating keys against the
# option index. The walker only recurses into "structured" containers — a key
# K at parent section P has a structured child section (K if P==top_level else
# P.K) only when that child section exists in the index. Free-form containers
# (retry_policy, fallbacks, environment_variables, model_group_alias, ...)
# have no indexed child section, so their sub-keys are data, not config options.
# ---------------------------------------------------------------------------

def walk_and_check_keys(
    obj: Any,
    parent_section: str,
    path: str,
    findings: List[Finding],
    ctx: Dict[str, Any],
) -> None:
    if isinstance(obj, dict):
        for k, v in obj.items():
            key_path = f"{path}.{k}" if path else str(k)
            _check_one_key(k, v, parent_section, key_path, findings, ctx)
            child_section = (
                k if parent_section == "top_level" else f"{parent_section}.{k}"
            )
            if child_section in ctx["sections"]:
                if isinstance(v, dict):
                    walk_and_check_keys(
                        v, child_section, key_path, findings, ctx
                    )
                elif isinstance(v, list):
                    for item in v:
                        if isinstance(item, dict):
                            walk_and_check_keys(
                                item, child_section, key_path, findings, ctx
                            )
    elif isinstance(obj, list):
        for item in obj:
            if isinstance(item, dict):
                walk_and_check_keys(item, parent_section, path, findings, ctx)


def _check_one_key(
    k: str,
    v: Any,
    parent_section: str,
    key_path: str,
    findings: List[Finding],
    ctx: Dict[str, Any],
) -> None:
    entries = ctx["options_by_key"].get(k)
    if not entries:
        findings.append(
            (
                "a",
                "FAIL",
                key_path,
                f"unknown key '{k}' (not in config-yaml.option-index.json)",
            )
        )
        return
    matching = [e for e in entries if e["section"] == parent_section]
    if not matching:
        expected = ", ".join(sorted({e["section"] for e in entries}))
        findings.append(
            (
                "a",
                "FAIL",
                key_path,
                f"key '{k}' under wrong section '{parent_section}' "
                f"(expected: {expected})",
            )
        )
        return
    entry = matching[0]

    # (d) deprecated keys -> WARN with replacement.
    if entry.get("deprecated") is True:
        repl = entry.get("replacement") or "(no replacement documented)"
        findings.append(
            (
                "d",
                "WARN",
                key_path,
                f"deprecated key '{k}' — use {repl}",
            )
        )

    # (e) requires_db keys (in-memory mode).
    if entry.get("requires_db") is True and ctx["mode"] == "in-memory":
        if k == "disable_spend_logs" and v is True:
            findings.append(
                (
                    "e",
                    "WARN",
                    key_path,
                    "requires_db=true; set true intentionally as compensating "
                    "control for no DB (allowed)",
                )
            )
        else:
            findings.append(
                (
                    "e",
                    "FAIL",
                    key_path,
                    f"requires_db=true key '{k}' set in in-memory mode "
                    f"(needs Postgres)",
                )
            )

    # (f) requires_redis keys (in-memory mode).
    if entry.get("requires_redis") is True and ctx["mode"] == "in-memory":
        findings.append(
            (
                "f",
                "FAIL",
                key_path,
                f"requires_redis=true key '{k}' set in in-memory mode "
                f"(needs Redis)",
            )
        )


# ---------------------------------------------------------------------------
# Check (b): provider prefix.
# ---------------------------------------------------------------------------

def check_provider_prefix(
    model_list: List[Dict[str, Any]], findings: List[Finding], ctx: Dict[str, Any]
) -> None:
    for i, entry in enumerate(model_list):
        params = entry.get("litellm_params") or {}
        model = params.get("model")
        if not isinstance(model, str) or not model:
            findings.append(
                (
                    "b",
                    "FAIL",
                    f"model_list[{i}].litellm_params.model",
                    "missing or non-string model value",
                )
            )
            continue
        prefix = model.split("/", 1)[0] + "/"
        if prefix not in ctx["valid_prefixes"]:
            findings.append(
                (
                    "b",
                    "FAIL",
                    f"model_list[{i}].litellm_params.model",
                    f"unknown provider prefix '{prefix}' (model='{model}')",
                )
            )


# ---------------------------------------------------------------------------
# Check (c): fallback targets resolve.
# ---------------------------------------------------------------------------

def check_fallbacks(
    config: Dict[str, Any], findings: List[Finding]
) -> None:
    model_list = config.get("model_list") or []
    if not isinstance(model_list, list):
        return
    model_names = {
        e.get("model_name")
        for e in model_list
        if isinstance(e, dict) and isinstance(e.get("model_name"), str)
    }
    router = config.get("router_settings") or {}
    if not isinstance(router, dict):
        return
    fallbacks = router.get("fallbacks")
    if not isinstance(fallbacks, list):
        return
    for i, item in enumerate(fallbacks):
        if not isinstance(item, dict):
            continue
        for src, targets in item.items():
            if src not in model_names:
                findings.append(
                    (
                        "c",
                        "FAIL",
                        f"router_settings.fallbacks[{i}].{src}",
                        f"fallback source '{src}' is not a model_name in "
                        f"model_list",
                    )
                )
            if isinstance(targets, list):
                for tgt in targets:
                    if tgt not in model_names:
                        findings.append(
                            (
                                "c",
                                "FAIL",
                                f"router_settings.fallbacks[{i}].{src}",
                                f"fallback target '{tgt}' is not a model_name "
                                f"in model_list",
                            )
                        )


# ---------------------------------------------------------------------------
# Check (g): env var resolution + master_key indirection.
# ---------------------------------------------------------------------------

def check_env_vars(
    config: Dict[str, Any], findings: List[Finding], ctx: Dict[str, Any]
) -> None:
    refs = extract_env_refs(config)
    known = ctx["known_env_vars"] | ctx["project_defined_vars"]
    seen: Set[str] = set()
    for var, key_path in refs:
        if var in seen:
            continue
        seen.add(var)
        if var in known:
            continue
        correct = is_misspelling_of_known(var, ctx["known_env_vars"])
        if correct:
            findings.append(
                (
                    "g",
                    "FAIL",
                    key_path,
                    f"env var '{var}' looks like a misspelling of known "
                    f"built-in '{correct}'",
                )
            )
        else:
            findings.append(
                (
                    "g",
                    "WARN",
                    key_path,
                    f"env var '{var}' is not a known LiteLLM built-in "
                    f"(may be project-defined)",
                )
            )

    # Deprecated env vars referenced via os.environ/ -> WARN (check d).
    dep_env = ctx["deprecated_env_vars"]
    for var, key_path in refs:
        if var in dep_env:
            repl = dep_env[var] or "(no replacement documented)"
            findings.append(
                ("d", "WARN", key_path, f"deprecated env var '{var}' — use {repl}")
            )

    # DB-requiring env vars referenced or set -> check (e) in in-memory mode.
    if ctx["mode"] == "in-memory":
        db_env = ctx["db_env_vars"]
        for var, key_path in refs:
            if var in db_env:
                findings.append(
                    (
                        "e",
                        "FAIL",
                        key_path,
                        f"DB-requiring env var '{var}' referenced in "
                        f"in-memory mode (needs Postgres)",
                    )
                )
        # environment_variables: section (top-level) injects env vars at runtime.
        env_section = config.get("environment_variables")
        if isinstance(env_section, dict):
            for var in env_section:
                if var in db_env:
                    findings.append(
                        (
                            "e",
                            "FAIL",
                            f"environment_variables.{var}",
                            f"DB-requiring env var '{var}' set in "
                            f"in-memory mode (needs Postgres)",
                        )
                    )
                if var.startswith("REDIS_"):
                    findings.append(
                        (
                            "f",
                            "FAIL",
                            f"environment_variables.{var}",
                            f"Redis env var '{var}' set in in-memory mode "
                            f"(needs Redis)",
                        )
                    )
        # REDIS_* referenced via os.environ/ -> check (f).
        for var, key_path in refs:
            if var.startswith("REDIS_"):
                findings.append(
                    (
                        "f",
                        "FAIL",
                        key_path,
                        f"Redis env var '{var}' referenced in in-memory mode "
                        f"(needs Redis)",
                    )
                )

    # master_key must resolve to os.environ/LITELLM_MASTER_KEY (security note).
    general = config.get("general_settings") or {}
    if isinstance(general, dict):
        mk = general.get("master_key")
        if mk is None:
            findings.append(
                (
                    "g",
                    "FAIL",
                    "general_settings.master_key",
                    "master_key is missing (must be "
                    "os.environ/LITELLM_MASTER_KEY)",
                )
            )
        elif not (isinstance(mk, str) and mk.strip() == "os.environ/LITELLM_MASTER_KEY"):
            findings.append(
                (
                    "g",
                    "FAIL",
                    "general_settings.master_key",
                    "master_key must resolve to "
                    "os.environ/LITELLM_MASTER_KEY (hardcoded/other value "
                    "is a secret-hygiene violation)",
                )
            )


# ---------------------------------------------------------------------------
# Checks (h) + (i): openai/ and anthropic/ api_base rules.
# ---------------------------------------------------------------------------

def check_openai_api_base(
    model_list: List[Dict[str, Any]], findings: List[Finding]
) -> None:
    for i, entry in enumerate(model_list):
        params = entry.get("litellm_params") or {}
        model = params.get("model")
        if not isinstance(model, str) or not model.startswith("openai/"):
            continue
        base = params.get("api_base")
        key_path = f"model_list[{i}].litellm_params.api_base"
        if not isinstance(base, str) or not base:
            findings.append(
                (
                    "h",
                    "FAIL",
                    f"model_list[{i}].litellm_params.api_base",
                    f"openai/ entry (model='{model}') missing api_base "
                    f"(must include /v1 postfix)",
                )
            )
        else:
            norm = base.rstrip("/")
            if norm.endswith("/v1"):
                pass  # correct
            elif "/v1/" in norm:
                findings.append(
                    (
                        "h",
                        "FAIL",
                        key_path,
                        f"openai/ api_base '{base}' appends an endpoint path "
                        f"after /v1 (the openai-client adds endpoints itself)",
                    )
                )
            else:
                findings.append(
                    (
                        "h",
                        "FAIL",
                        key_path,
                        f"openai/ api_base '{base}' missing /v1 postfix "
                        f"(causes Not Found Error)",
                    )
                )
        if not params.get("api_key"):
            findings.append(
                (
                    "h",
                    "FAIL",
                    f"model_list[{i}].litellm_params.api_key",
                    f"openai/ entry (model='{model}') missing api_key "
                    f"(openai-client requires a key; use hosted_vllm/ for "
                    f"keyless endpoints)",
                )
            )


def check_anthropic_api_base(
    model_list: List[Dict[str, Any]], findings: List[Finding], disable_suffix: bool
) -> None:
    for i, entry in enumerate(model_list):
        params = entry.get("litellm_params") or {}
        model = params.get("model")
        if not isinstance(model, str) or not model.startswith("anthropic/"):
            continue
        base = params.get("api_base")
        if not isinstance(base, str) or not base:
            continue  # missing api_base is not an (i) failure per the doc
        if base.rstrip("/").endswith("/v1/messages") and not disable_suffix:
            findings.append(
                (
                    "i",
                    "WARN",
                    f"model_list[{i}].litellm_params.api_base",
                    f"anthropic/ api_base '{base}' ends with /v1/messages; "
                    f"LiteLLM auto-appends /v1/messages (would double-append). "
                    f"Set LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true or strip the "
                    f"suffix",
                )
            )


def anthropic_suffix_disabled(config: Dict[str, Any]) -> bool:
    """True if LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX is set true via the config
    (environment_variables section, os.environ/ reference) or the live env."""
    env_section = config.get("environment_variables")
    if isinstance(env_section, dict):
        val = env_section.get("LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX")
        if val in (True, "true", "True", "1"):
            return True
    for var, _ in extract_env_refs(config):
        if var == "LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX":
            return True
    return os.environ.get("LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX", "").lower() in (
        "1",
        "true",
    )


# ---------------------------------------------------------------------------
# Reporting.
# ---------------------------------------------------------------------------

CHECK_LABELS = {
    "a": "Key existence + section",
    "b": "Provider prefix",
    "c": "Fallback targets resolve",
    "d": "Deprecated keys",
    "e": "DB-requiring keys (in-memory)",
    "f": "Redis-requiring keys",
    "g": "Env var resolution + master_key",
    "h": "openai/ api_base + api_key",
    "i": "anthropic/ api_base suffix",
}


def report(findings: List[Finding]) -> int:
    by_check: Dict[str, List[Finding]] = {k: [] for k in CHECK_LABELS}
    for letter, sev, key_path, msg in findings:
        by_check.setdefault(letter, []).append((letter, sev, key_path, msg))

    print("LiteLLM config validation report")
    print("=" * 60)

    total_fail = 0
    total_warn = 0
    for letter in CHECK_LABELS:
        items = by_check.get(letter, [])
        fails = [f for f in items if f[1] == "FAIL"]
        warns = [f for f in items if f[1] == "WARN"]
        if fails:
            verdict = "FAIL"
            total_fail += len(fails)
        elif warns:
            verdict = "WARN"
            total_warn += len(warns)
        else:
            verdict = "PASS"
        label = CHECK_LABELS[letter]
        print(f"  ({letter}) {label:<34} {verdict}")

    print()
    if findings:
        print("Findings detail:")
        for letter, sev, key_path, msg in findings:
            print(f"  ({letter}) {sev}: {key_path} — {msg}")
    else:
        print("No findings.")

    print()
    overall = "PASS" if total_fail == 0 else "FAIL"
    print(
        f"Overall verdict: {overall} ({total_fail} FAIL, {total_warn} WARN)"
    )
    return 0 if total_fail == 0 else 1


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate a LiteLLM config.yaml against the schema indexes.",
    )
    parser.add_argument("--config", required=True, help="path to config.yaml")
    parser.add_argument(
        "--schemas-dir",
        dest="schemas_dir",
        help="path to the schemas directory (e.g. docs/litellm/schemas)",
    )
    parser.add_argument(
        "--schemas",
        dest="schemas_dir",
        help="alias for --schemas-dir",
    )
    parser.add_argument(
        "--mode",
        choices=["in-memory", "db-backed"],
        default="in-memory",
        help="deployment mode (default: in-memory)",
    )
    args = parser.parse_args(argv)

    if not args.schemas_dir:
        parser.error("--schemas-dir (or --schemas) is required")

    config_path = args.config
    schemas_dir = args.schemas_dir
    if not os.path.isfile(config_path):
        sys.stderr.write(f"ERROR: config not found: {config_path}\n")
        return 2

    option_index = load_index(
        os.path.join(schemas_dir, "config-yaml.option-index.json"),
        "config-yaml.option-index",
    )
    provider_index = load_index(
        os.path.join(schemas_dir, "provider-fields.index.json"),
        "provider-fields.index",
    )
    env_index = load_index(
        os.path.join(schemas_dir, "env-vars.index.json"), "env-vars.index"
    )

    options = option_index.get("options", []) if isinstance(option_index, dict) else []
    providers = (
        provider_index.get("providers", []) if isinstance(provider_index, dict) else []
    )
    env_vars = env_index.get("env_vars", []) if isinstance(env_index, dict) else []

    options_by_key, sections = build_option_lookup(options)
    valid_prefixes = collect_valid_prefixes(providers)
    known_env_vars = collect_env_var_names(env_vars)
    db_env_vars = env_var_requires_db(env_vars)
    dep_env_vars = env_var_deprecated(env_vars)

    # Project-defined vars referenced in the canonical deployment config.
    repo_root = os.path.dirname(
        os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    )
    infra_config = os.path.join(repo_root, "infra", "litellm", "config.yaml")
    project_defined = collect_project_defined_vars(infra_config)

    try:
        with open(config_path, "r", encoding="utf-8") as fh:
            config = yaml.safe_load(fh)
    except yaml.YAMLError as exc:
        sys.stderr.write(f"ERROR: failed to parse config {config_path}: {exc}\n")
        return 2
    except OSError as exc:
        sys.stderr.write(f"ERROR: cannot read config {config_path}: {exc}\n")
        return 2

    if not isinstance(config, dict):
        sys.stderr.write(
            f"ERROR: config {config_path} is not a YAML mapping at top level\n"
        )
        return 2

    # Resolve `include:` directives so a parent config validates its merged
    # children (e.g. config.yaml -> models.yaml) as a single merged config.
    config = resolve_includes(config, config_path)

    ctx: Dict[str, Any] = {
        "options_by_key": options_by_key,
        "sections": sections,
        "valid_prefixes": valid_prefixes,
        "known_env_vars": known_env_vars,
        "db_env_vars": db_env_vars,
        "deprecated_env_vars": dep_env_vars,
        "project_defined_vars": project_defined,
        "mode": args.mode,
    }

    findings: List[Finding] = []

    # (a)(d)(e)(f) key walk
    walk_and_check_keys(config, "top_level", "", findings, ctx)

    model_list = config.get("model_list") or []
    if not isinstance(model_list, list):
        model_list = []

    # (b) provider prefix
    check_provider_prefix(model_list, findings, ctx)
    # (c) fallbacks
    check_fallbacks(config, findings)
    # (g) env vars + master_key
    check_env_vars(config, findings, ctx)
    # (h) openai/ api_base
    check_openai_api_base(model_list, findings)
    # (i) anthropic/ api_base
    check_anthropic_api_base(
        model_list, findings, anthropic_suffix_disabled(config)
    )

    print(f"Config: {config_path}")
    print(f"Mode: {args.mode}")
    print(f"Schemas: {schemas_dir}")
    print()
    return report(findings)


if __name__ == "__main__":
    sys.exit(main())
