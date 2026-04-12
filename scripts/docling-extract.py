#!/usr/bin/env python3
"""Reference Docling wrapper for PDF structural extraction."""

from __future__ import annotations

import argparse
import json
import os
import sys
from importlib import metadata as importlib_metadata
from pathlib import Path
from typing import Any

DEFAULT_ARTIFACTS_PATH = Path.home() / ".cache" / "docling" / "models"
SUCCESS_STATUSES = {"success"}
PARTIAL_SUCCESS_STATUSES = {"partial_success"}
SECTION_LABELS = {
    "title",
    "subtitle",
    "section",
    "section_header",
    "page_header",
    "page_title",
}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run Docling structural extraction for one PDF and emit normalized JSON.",
    )
    parser.add_argument("--input", help="Input PDF file path.")
    parser.add_argument("--output", help="Output JSON file path.")
    parser.add_argument(
        "--strategy",
        choices=("standard", "force_full_page_ocr", "fast_table_structure"),
        help="Processing strategy name.",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        help="Document processing timeout in seconds.",
    )
    parser.add_argument(
        "--version",
        action="store_true",
        help="Print the installed docling version and exit.",
    )
    return parser


def installed_docling_version() -> str:
    try:
        return importlib_metadata.version("docling")
    except importlib_metadata.PackageNotFoundError:
        try:
            import docling  # type: ignore
        except ImportError as exc:
            raise ImportError(f"docling import unavailable: {exc}") from exc

        version = getattr(docling, "__version__", None)
        if not version:
            raise RuntimeError("docling is installed but no version metadata was found")
        return str(version)


def require_artifacts_path() -> Path:
    raw_path = os.environ.get("DOCLING_ARTIFACTS_PATH")
    artifacts_path = Path(raw_path).expanduser() if raw_path else DEFAULT_ARTIFACTS_PATH
    if not artifacts_path.is_dir():
        raise RuntimeError(
            "Docling model artifacts are unavailable at "
            f"{artifacts_path}. Prefetch them with `docling-tools models download` "
            "or set DOCLING_ARTIFACTS_PATH to an existing model directory."
        )
    return artifacts_path


def force_offline_artifacts(artifacts_path: Path) -> None:
    os.environ.setdefault("DOCLING_ARTIFACTS_PATH", str(artifacts_path))
    os.environ.setdefault("HF_HUB_OFFLINE", "1")
    os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")


def load_docling_api() -> dict[str, Any]:
    try:
        from docling.backend.docling_parse_backend import DoclingParseDocumentBackend
        from docling.datamodel.base_models import ConversionStatus, InputFormat
        from docling.datamodel.pipeline_options import (
            OcrAutoOptions,
            PdfPipelineOptions,
            TableFormerMode,
            TableStructureOptions,
        )
        from docling.document_converter import DocumentConverter, PdfFormatOption
    except ImportError as exc:
        raise ImportError(f"docling import unavailable: {exc}") from exc

    return {
        "ConversionStatus": ConversionStatus,
        "DoclingParseDocumentBackend": DoclingParseDocumentBackend,
        "DocumentConverter": DocumentConverter,
        "InputFormat": InputFormat,
        "OcrAutoOptions": OcrAutoOptions,
        "PdfFormatOption": PdfFormatOption,
        "PdfPipelineOptions": PdfPipelineOptions,
        "TableFormerMode": TableFormerMode,
        "TableStructureOptions": TableStructureOptions,
    }


def validate_runtime_args(args: argparse.Namespace) -> tuple[Path, Path]:
    if not args.input or not args.output or not args.strategy or args.timeout is None:
        raise ValueError(
            "--input, --output, --strategy, and --timeout are required unless --version is used"
        )

    input_path = Path(args.input).expanduser()
    if not input_path.is_file():
        raise ValueError(f"input PDF not found: {input_path}")
    if input_path.suffix.lower() != ".pdf":
        raise ValueError(f"input must be a .pdf file: {input_path}")

    output_path = Path(args.output).expanduser()
    if output_path.is_dir():
        raise ValueError(f"output path must be a file, not a directory: {output_path}")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    return input_path, output_path


def build_pipeline_options(api: dict[str, Any], strategy: str, timeout: float, artifacts_path: Path) -> Any:
    pdf_pipeline_options = api["PdfPipelineOptions"](
        artifacts_path=artifacts_path,
        document_timeout=timeout,
    )
    pdf_pipeline_options.do_ocr = True
    pdf_pipeline_options.do_table_structure = True
    pdf_pipeline_options.ocr_options = api["OcrAutoOptions"]()
    pdf_pipeline_options.table_structure_options = api["TableStructureOptions"](
        do_cell_matching=True,
    )

    if strategy == "standard":
        pdf_pipeline_options.table_structure_options.mode = api["TableFormerMode"].ACCURATE
    elif strategy == "force_full_page_ocr":
        pdf_pipeline_options.table_structure_options.mode = api["TableFormerMode"].ACCURATE
        pdf_pipeline_options.ocr_options.force_full_page_ocr = True
    elif strategy == "fast_table_structure":
        pdf_pipeline_options.table_structure_options.mode = api["TableFormerMode"].FAST
    else:
        raise ValueError(f"unsupported strategy: {strategy}")

    return pdf_pipeline_options


def convert_pdf(api: dict[str, Any], input_path: Path, strategy: str, timeout: float, artifacts_path: Path) -> Any:
    pipeline_options = build_pipeline_options(api, strategy, timeout, artifacts_path)
    converter = api["DocumentConverter"](
        format_options={
            api["InputFormat"].PDF: api["PdfFormatOption"](
                pipeline_options=pipeline_options,
                backend=api["DoclingParseDocumentBackend"],
            )
        }
    )
    return converter.convert(str(input_path), raises_on_error=False)


def status_name(status: Any) -> str:
    if hasattr(status, "value"):
        return str(status.value)
    return str(status).lower()


def error_messages(result: Any) -> list[str]:
    errors = getattr(result, "errors", None) or []
    messages: list[str] = []
    for item in errors:
        message = getattr(item, "error_message", None)
        messages.append(str(message) if message else str(item))
    return messages


def serialize_exported_document(result: Any) -> tuple[Any, dict[str, Any]]:
    document = result.document
    doc_dict = document.export_to_dict()
    return document, doc_dict


def page_numbers_for(node: Any) -> list[int]:
    page_numbers: list[int] = []

    def visit(value: Any) -> None:
        if isinstance(value, dict):
            for key, nested in value.items():
                if key in {"page_no", "page", "page_index"} and isinstance(nested, int):
                    page_numbers.append(nested)
                else:
                    visit(nested)
        elif isinstance(value, list):
            for nested in value:
                visit(nested)

    visit(node)
    return sorted(set(page_numbers))


def heading_text_for(node: dict[str, Any]) -> str:
    for key in ("heading", "title", "text", "name"):
        value = node.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return ""


def normalize_sections(doc_dict: dict[str, Any]) -> list[dict[str, Any]]:
    seen: set[tuple[str, str, int, int]] = set()
    sections: list[dict[str, Any]] = []

    def visit(value: Any) -> None:
        if isinstance(value, dict):
            label = str(value.get("label", "")).lower()
            heading = heading_text_for(value)
            looks_like_section = (
                label in SECTION_LABELS
                or "section_ref" in value
                or ("heading" in value and heading)
            )
            if looks_like_section and heading:
                pages = page_numbers_for(value)
                section_ref = str(
                    value.get("section_ref") or value.get("self_ref") or value.get("id") or heading
                )
                page_start = pages[0] if pages else 1
                page_end = pages[-1] if pages else page_start
                key = (section_ref, heading, page_start, page_end)
                if key not in seen:
                    seen.add(key)
                    sections.append(
                        {
                            "section_ref": section_ref,
                            "heading": heading,
                            "page_start": page_start,
                            "page_end": page_end,
                        }
                    )
            for nested in value.values():
                visit(nested)
        elif isinstance(value, list):
            for nested in value:
                visit(nested)

    visit(doc_dict)
    return sections


def table_title_for(node: dict[str, Any]) -> str | None:
    for key in ("title", "caption", "name"):
        value = node.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return None


def normalize_rows(value: Any) -> list[Any]:
    if isinstance(value, list):
        return value
    return []


def normalize_tables(doc_dict: dict[str, Any]) -> list[dict[str, Any]]:
    seen: set[str] = set()
    tables: list[dict[str, Any]] = []

    def visit(value: Any) -> None:
        if isinstance(value, dict):
            label = str(value.get("label", "")).lower()
            has_table_shape = label == "table" or "table_key" in value or (
                "headers" in value and "rows" in value
            )
            if has_table_shape:
                table_key = str(value.get("table_key") or value.get("self_ref") or value.get("id") or "")
                if not table_key:
                    table_key = f"table-{len(tables) + 1}"
                if table_key not in seen:
                    seen.add(table_key)
                    pages = page_numbers_for(value)
                    page_start = pages[0] if pages else 1
                    page_end = pages[-1] if pages else page_start
                    tables.append(
                        {
                            "table_key": table_key,
                            "title": table_title_for(value),
                            "page_start": page_start,
                            "page_end": page_end,
                            "headers": normalize_rows(value.get("headers")),
                            "rows": normalize_rows(value.get("rows")),
                        }
                    )
            for nested in value.values():
                visit(nested)
        elif isinstance(value, list):
            for nested in value:
                visit(nested)

    visit(doc_dict)
    return tables


def infer_page_count(result: Any, document: Any, sections: list[dict[str, Any]], tables: list[dict[str, Any]]) -> int:
    num_pages = getattr(document, "num_pages", None)
    if callable(num_pages):
        try:
            return int(num_pages())
        except Exception:
            pass

    input_pages = getattr(getattr(result, "input", None), "page_count", None)
    if isinstance(input_pages, int) and input_pages > 0:
        return input_pages

    candidate_pages = [
        entry.get("page_end", 0)
        for entry in [*sections, *tables]
        if isinstance(entry.get("page_end"), int)
    ]
    return max(candidate_pages, default=0)


def infer_image_count(doc_dict: dict[str, Any]) -> int:
    count = 0

    def visit(value: Any) -> None:
        nonlocal count
        if isinstance(value, dict):
            label = str(value.get("label", "")).lower()
            if label in {"figure", "picture", "image"}:
                count += 1
            for nested in value.values():
                visit(nested)
        elif isinstance(value, list):
            for nested in value:
                visit(nested)

    visit(doc_dict)
    return count


def infer_has_ocr_content(strategy: str, doc_dict: dict[str, Any]) -> bool:
    if strategy == "force_full_page_ocr":
        return True

    def visit(value: Any) -> bool:
        if isinstance(value, dict):
            for key, nested in value.items():
                lowered = key.lower()
                if lowered in {"ocr", "ocr_used", "ocr_applied", "from_ocr"} and bool(nested):
                    return True
                if visit(nested):
                    return True
        elif isinstance(value, list):
            for nested in value:
                if visit(nested):
                    return True
        return False

    return visit(doc_dict)


def normalize_output(result: Any, version: str, strategy: str, input_path: Path) -> dict[str, Any]:
    document, doc_dict = serialize_exported_document(result)
    markdown = document.export_to_markdown()
    sections = normalize_sections(doc_dict)
    tables = normalize_tables(doc_dict)
    page_count = infer_page_count(result, document, sections, tables)

    warnings = error_messages(result)
    if not markdown.strip():
        warnings.append("Docling produced empty markdown output.")
    if not sections:
        warnings.append("No structured sections were detected in the exported document.")
    if not tables:
        warnings.append("No structured tables were detected in the exported document.")

    quality = {
        "table_count": len(tables),
        "section_count": len(sections),
        "image_count": infer_image_count(doc_dict),
        "empty_page_count": 0 if markdown.strip() else max(page_count, 1),
        "has_ocr_content": infer_has_ocr_content(strategy, doc_dict),
        "warnings": warnings,
    }

    metadata = {
        "document_title": input_path.stem,
        "document_language": None,
        "source": "docling-wrapper",
        "conversion_status": status_name(getattr(result, "status", "unknown")),
        "docling_version": version,
        "input_path": str(input_path),
        "docling_document": doc_dict,
    }

    return {
        "markdown": markdown,
        "sections": sections,
        "tables": tables,
        "metadata": metadata,
        "page_count": page_count,
        "quality": quality,
    }


def write_output(output_path: Path, payload: dict[str, Any]) -> None:
    output_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    try:
        if args.version:
            print(installed_docling_version())
            return 0

        input_path, output_path = validate_runtime_args(args)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 2
    except ImportError as exc:
        print(str(exc), file=sys.stderr)
        return 3

    try:
        artifacts_path = require_artifacts_path()
        force_offline_artifacts(artifacts_path)
        version = installed_docling_version()
        api = load_docling_api()
    except ImportError as exc:
        print(str(exc), file=sys.stderr)
        return 3
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        return 1

    try:
        result = convert_pdf(api, input_path, args.strategy, args.timeout, artifacts_path)
        status = status_name(getattr(result, "status", "unknown"))
        if status in PARTIAL_SUCCESS_STATUSES:
            errors = "; ".join(error_messages(result)) or "document conversion reached partial_success"
            print(errors, file=sys.stderr)
            return 1
        if status not in SUCCESS_STATUSES:
            errors = "; ".join(error_messages(result)) or f"document conversion failed with status `{status}`"
            print(errors, file=sys.stderr)
            return 1

        payload = normalize_output(result, version, args.strategy, input_path)
        write_output(output_path, payload)
        return 0
    except ImportError as exc:
        print(f"docling import unavailable: {exc}", file=sys.stderr)
        return 3
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
