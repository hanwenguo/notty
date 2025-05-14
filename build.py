#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# dependencies = [
#     "ripgrepy",
# ]
# ///
import argparse
import os
import re
import shutil
import subprocess
from pathlib import Path
import http.server
import socketserver
import contextlib
import json
from ripgrepy import Ripgrepy

# --- Configuration ---
ROOT_DIR = Path(__file__).parent.resolve()
TYPST_EXE = "typst"
TYPST_SUBCOMMAND = "compile"

TYPST_DIR = ROOT_DIR / "typ"
HTML_DIR = ROOT_DIR / "html"
PDF_DIR = HTML_DIR / "pdf"
PUBLIC_DIR = ROOT_DIR / "public"
TEMPLATES_DIR = ROOT_DIR / "_template"

ID_PATH_MAP_FILE = TYPST_DIR / "id_path_map.json"
PDF_TEMPLATE_SRC = TEMPLATES_DIR / "template-paged.typ"
HTML_TEMPLATE_SRC = TEMPLATES_DIR / "template-html.typ"
ACTIVE_TEMPLATE_DST = TEMPLATES_DIR / "template.typ"

TYPST_COMMON_FLAGS = ["--root", str(ROOT_DIR)]
TYPST_HTML_FLAGS = TYPST_COMMON_FLAGS + ["--features", "html", "--format", "html"]
TYPST_PDF_FLAGS = TYPST_COMMON_FLAGS + ["--format", "pdf"]

# Regex to extract timestamp (YYYYMMDDTHHMMSS) from start of filename
FILENAME_TIMESTAMP_RE = re.compile(r"^(\d{8}T\d{6})(.*)?\.typ$")

ID_PATH_MAP = {}
PATH_ID_MAP = {}

with open(ID_PATH_MAP_FILE, "r", encoding="utf-8") as f:
    ID_PATH_MAP = json.load(f)
    PATH_ID_MAP = {v: k for k, v in ID_PATH_MAP.items()}

# --- Helper Functions ---

def run_command(cmd_list, cwd=None, print_stdout=True, input_stdin: str | None = None):
    """Runs a command and returns the result object.
    Optionally prints stdout. Always prints stderr if present (on success or failure).
    """
    print(f"Running: {' '.join(map(str, cmd_list))}")
    try:
        result = subprocess.run(cmd_list, check=True, capture_output=True, text=True, cwd=cwd, input=input_stdin)
        if print_stdout and result.stdout:
            print("Command STDOUT:")
            print(result.stdout)
        if result.stderr: # Print stderr if any, even on success
            print("Command STDERR:")
            print(result.stderr)
        return result
    except subprocess.CalledProcessError as e:
        print(f"Error running command: {e}")
        if e.stdout: # Print stdout from error object
            print("Command STDOUT (on error):")
            print(e.stdout)
        if e.stderr: # Print stderr from error object
            print("Command STDERR (on error):")
            print(e.stderr)
        raise # Re-raise the exception to stop the build if needed

# def get_identifier_from_filename(filename):
#     """Extracts the identifier from a filename using regex."""
#     if filename == "index.typ":
#         return "index"
#     match = FILENAME_TIMESTAMP_RE.match(filename)
#     if match:
#         return match.group(1)  # Return the timestamp part
#     return None

def get_identifier_from_path(path: Path):
    """Extracts the identifier from a path."""
    project_absolute_path = f"/{path.relative_to(ROOT_DIR)}"
    return PATH_ID_MAP.get(project_absolute_path, None)

def get_typst_sources():
    """Finds all .typ files in the TYPST_DIR."""
    return list(TYPST_DIR.glob("*.typ"))

def get_id_to_path_map() -> dict[str, str]:
    """
    Scans a directory recursively for .typ files and returns a mapping
    of their IDs to their relative paths.
    """
    id_to_path = {}
    query = r'^[[:blank:]]*identifier[[:blank:]]*:[[:blank:]]*"([^\"]+?)",?[[:blank:]]*$'
    rg = (Ripgrepy(query, str(TYPST_DIR))
          .ignore_case()
          .with_filename()
          .no_messages()
          .glob("*.typ")
          .null()
          .only_matching()
          .replace('$1'))
    raw_result = rg.run().as_string.split("\n")
    results_maybe_duplicated = [line.split("\x00") for line in raw_result if line]
    results_maybe_duplicated.reverse() # Reverse to keep the first found
    results = {item[0]: item[1] for item in results_maybe_duplicated}
    
    for path, id in results.items():
        path = Path(path).relative_to(ROOT_DIR)
        id_to_path[id] = f"/{str(path)}"
        
    return id_to_path

def write_id_to_path_map():
    """
    Writes the ID to path mapping to a JSON file.
    """
    id_to_path = get_id_to_path_map()
    if not id_to_path:
        print("No .typ files found in the specified directory.")
        return

    try:
        with open(ID_PATH_MAP_FILE, "w", encoding="utf-8") as f:
            json.dump(id_to_path, f, indent=2)
        print(f"Successfully updated ID to path mapping in {ID_PATH_MAP_FILE}")
    except IOError as e:
        print(f"Error writing ID to path mapping: {e}")
    print(f"ID to path mapping written to {ID_PATH_MAP_FILE}")

def get_target_filename(source_path, output_dir, extension):
    """Determines the target filename based on the timestamp."""
    target_id = PATH_ID_MAP.get(source_path, None)
    if not target_id:
        match = FILENAME_TIMESTAMP_RE.match(source_path.name)
        if match:
            timestamp = match.group(1)
            return output_dir / f"{timestamp}{extension}"
        else:
            # Fallback for files not matching the timestamp pattern (optional)
            # You might want to skip these or handle them differently
            print(f"Warning: Skipping {source_path.name} - does not match expected timestamp pattern.")
            return None
    else:
        # If the ID is found, use it to determine the target filename
        target_filename = f"{target_id}{extension}"
        return output_dir / target_filename

def copy_public_assets():
    """Copies contents of PUBLIC_DIR to HTML_DIR."""
    if not PUBLIC_DIR.exists() or not any(PUBLIC_DIR.iterdir()):
        print(f"Notice: {PUBLIC_DIR} is empty or does not exist, skipping copy.")
        return

    print(f"Copying contents of {PUBLIC_DIR} to {HTML_DIR}...")
    HTML_DIR.mkdir(parents=True, exist_ok=True)
    # Use copytree with dirs_exist_ok=True for robustness
    shutil.copytree(PUBLIC_DIR, HTML_DIR, dirs_exist_ok=True)


def prepare_template(template_type):
    """Copies the correct template file."""
    src_template = PDF_TEMPLATE_SRC if template_type == "pdf" else HTML_TEMPLATE_SRC
    if not src_template.exists():
        print(f"Error: Source template {src_template} not found.")
        return False
    print(f"Setting active template to {src_template.name}...")
    shutil.copy2(src_template, ACTIVE_TEMPLATE_DST)
    return True

# --- Build Functions ---

def backmatters_section_source(title, paths: list[Path]):
    """Generates a backmatters section source string."""
    ids = [get_identifier_from_path(path) for path in paths]
    urls = [f"\"denote:{id}\"" for id in ids if id]
    return f"(name: \"{title}\", urls: ({', '.join(urls)}{"" if len(urls) == 0 else ","}))"

def query_for_backlinks(file_path: Path) -> str:
    file_id = get_identifier_from_path(file_path)
    if file_id:
        regex = r'#ln\([[:blank:]]*"denote:(?<id>' + file_id + r')"[[:blank:]]*\)\[(?<text>.*?)\]'
        return regex
    return ""

def query_for_contexts(file_path: Path) -> str:
    file_id = get_identifier_from_path(file_path)
    if file_id:
        regex = r'#tr\([[:blank:]]*"denote:(?<id>' + file_id + r')"'
        return regex
    return ""

def query_to_paths(query: str) -> list[Path]:
    rg = Ripgrepy(query, str(TYPST_DIR))
    rg_with_options = (rg
                       .ignore_case()
                       .line_number()
                       .with_filename()
                       .no_messages()
                       .glob("*.typ")
                       .json())
    matches = rg_with_options.run().as_dict
    path_texts = map(lambda m: m["data"]["path"]["text"], matches)
    path_texts = set(path_texts)
    paths = map(lambda path_text: Path(path_text), path_texts)
    return list(paths)

def build_backmatters_section(title: str, query: str):
    """Builds a backmatters section from a query."""
    paths = query_to_paths(query)
    if not paths:
        # print(f"Warning: No paths found for query '{query}'")
        return None
    return backmatters_section_source(title, paths)

def build_backmatters(parts: list[tuple[str, str]]):
    built_sections = [build_backmatters_section(title, query) for title, query in parts]
    # Filter out None values
    sections_filtered = [section for section in built_sections if section is not None]
    parts_unwrapped = ", ".join(sections_filtered) + ","
    parts_arg = f"({"" if len(sections_filtered) == 0 else parts_unwrapped})"
    return f"#import \"/_template/template.typ\": backmatters\n#backmatters(parts: {parts_arg})"

def build_html():
    """Builds all HTML files."""
    print("--- Building HTML ---")
    if not prepare_template("html"):
        return
    copy_public_assets() # Copy assets first

    sources = get_typst_sources()
    if not sources:
        print("No source .typ files found.")
        return

    HTML_DIR.mkdir(parents=True, exist_ok=True)

    # Path to the HTML shell template that will wrap Typst's output
    html_shell_template_path = TEMPLATES_DIR / "template.html"
    if not html_shell_template_path.exists():
        print(f"Error: HTML shell template '{html_shell_template_path}' not found.")
        return

    try:
        with open(html_shell_template_path, "r", encoding="utf-8") as f:
            shell_template_content = f.read()
    except Exception as e:
        print(f"Error reading HTML shell template '{html_shell_template_path}': {e}")
        return

    for source_file in sources:
        final_html_target_path = get_target_filename(source_file, HTML_DIR, ".html")
        if not final_html_target_path:
            continue

        print(f"Compiling {source_file.name} -> {final_html_target_path.name}")
        # Command to make Typst output its minimal HTML to stdout
        cmd = [
            TYPST_EXE,
            TYPST_SUBCOMMAND,
            *TYPST_HTML_FLAGS,
            str(source_file),
            "-" # Output to stdout
        ]
        backmatters_cmd = [
            TYPST_EXE,
            TYPST_SUBCOMMAND,
            *TYPST_HTML_FLAGS,
            "--input", "no-numbering=true",
            "-", # Read from stdin
            "-" # Output to stdout
        ]
        try:
            # Run typst, get its output from stdout, don't print its stdout here
            result = run_command(cmd, print_stdout=False)
            typst_output_str = result.stdout

            if not typst_output_str:
                print(f"Warning: Typst produced no output for {source_file.name}")
                extracted_content = ""
            else:
                # Extract content from between <html> ... </html> tags
                # Expecting Typst to output: <!DOCTYPE html><html><section>...</section></html>
                content_match = re.search(r"<html[^>]*>(.*?)</html>", typst_output_str, re.DOTALL | re.IGNORECASE)
                if content_match:
                    extracted_content = content_match.group(1).strip()
                else:
                    # If <html> tags are not found, maybe the output is already just the fragment?
                    # Or it's an unexpected format. For now, assume the fragment is the whole output if <html> is missing.
                    # This might need adjustment based on actual Typst output.
                    print(f"Warning: Could not find <html> tags in Typst output for {source_file.name}. Using entire output as content.")
                    extracted_content = typst_output_str.strip()
                    # A more robust fallback might be to try and find <section> directly if <html> fails.
                    # For now, this assumes the content is either wrapped in <html> or is the direct fragment.

            # Extract title from <h1> tag in the extracted content
            title = final_html_target_path.stem # Default title is filename stem
            if extracted_content: # Only search for title if there's content
                title_match = re.search(r"<h1[^>]*><span class=\"taxon\">.*?</span>(.*?)<a.*?</a></h1>", extracted_content, re.IGNORECASE | re.DOTALL)
                if title_match:
                    raw_title = title_match.group(1).strip()
                    # Remove any HTML tags from the title for the <title> element
                    title = re.sub(r'<[^>]+>', '', raw_title).strip()
                    
            if extracted_content:
                backmatters_parts = [
                    ("Backlinks", query_for_backlinks(source_file)),
                    ("Contexts", query_for_contexts(source_file)),
                ]
                backmatters_section = build_backmatters(backmatters_parts)
                
                backmatters_result = run_command(backmatters_cmd, print_stdout=False, input_stdin=backmatters_section)
                backmatters_output_str = backmatters_result.stdout
                if backmatters_output_str:
                    # Extract content from between <html> ... </html> tags
                    backmatters_content_match = re.search(r"<html[^>]*>(.*?)</html>", backmatters_output_str, re.DOTALL | re.IGNORECASE)
                    if backmatters_content_match:
                        extracted_backmatters_content = backmatters_content_match.group(1).strip()
                        extracted_content += extracted_backmatters_content
                    else:
                        print(f"Warning: Could not find <html> tags in backmatters output for {source_file.name}.")
                else:
                    print(f"Warning: Backmatters command produced no output for {source_file.name}")

            # Prepare the final HTML by substituting into the shell template
            current_html_output = shell_template_content
            current_html_output = current_html_output.replace("<!-- contents goes here -->", extracted_content)
            current_html_output = current_html_output.replace("<title></title>", f"<title>{title}</title>")

            # Write the final composed HTML to the target file
            with open(final_html_target_path, "w", encoding="utf-8") as f_out:
                f_out.write(current_html_output)
            # print(f"Successfully generated {final_html_target_path}") # Already printed by "Compiling..."

        except subprocess.CalledProcessError:
            print(f"Failed to compile {source_file.name} for HTML fragment generation.")
            break # Stop on first error
        except Exception as e:
            print(f"An error occurred during HTML processing for {source_file.name}: {e}")
            # Consider if you want to 'continue' or 'break' here
            break

    print("--- HTML Build Complete ---")


def build_pdf():
    """Builds all PDF files."""
    print("--- Building PDF ---")
    if not prepare_template("pdf"):
        return

    sources = get_typst_sources()
    if not sources:
        print("No source .typ files found.")
        return

    PDF_DIR.mkdir(parents=True, exist_ok=True)

    for source_file in sources:
        target_file = get_target_filename(source_file, PDF_DIR, ".pdf")
        if not target_file:
            continue

        print(f"Compiling {source_file.name} -> {target_file.name}")
        cmd = [
            TYPST_EXE,
            TYPST_SUBCOMMAND,
            *TYPST_PDF_FLAGS,
            str(source_file),
            str(target_file)
        ]
        try:
            run_command(cmd, print_stdout=True) # Explicitly print Typst's stdout for PDF
        except subprocess.CalledProcessError:
            print(f"Failed to compile {source_file.name}")
            break # Stop on first error

    print("--- PDF Build Complete ---")


# --- Serve Function ---

def serve_html(port=8000):
    """Serves the HTML directory using Python's built-in HTTP server."""
    if not HTML_DIR.exists():
        print(f"Error: HTML directory '{HTML_DIR}' does not exist. Build first with 'python build.py html'.")
        return

    # SimpleHTTPRequestHandler serves files from the current working directory.
    # We use contextlib.chdir (Python 3.11+) for cleaner directory switching.
    # For older Python, os.chdir() can be used but requires careful handling.
    print(f"--- Serving HTML directory: {HTML_DIR} --- ")
    handler = http.server.SimpleHTTPRequestHandler

    # Try the specified port, then increment if it's in use
    while True:
        try:
            with socketserver.TCPServer(("", port), handler) as httpd:
                # Use contextlib.chdir if available (Python 3.11+)
                if hasattr(contextlib, 'chdir'):
                    with contextlib.chdir(HTML_DIR):
                        print(f"Serving at http://localhost:{port}")
                        print("Press Ctrl+C to stop.")
                        httpd.serve_forever()
                else:
                    # Fallback for older Python versions
                    original_cwd = os.getcwd()
                    try:
                        os.chdir(HTML_DIR)
                        print(f"Serving at http://localhost:{port}")
                        print("Press Ctrl+C to stop.")
                        httpd.serve_forever()
                    finally:
                        os.chdir(original_cwd) # Change back
            break # Exit loop if server starts successfully
        except OSError as e:
            if e.errno == 48: # Address already in use
                print(f"Port {port} is already in use, trying port {port + 1}...")
                port += 1
            else:
                print(f"Error starting server: {e}")
                break
        except KeyboardInterrupt:
            print("\nServer stopped.")
            break

# --- Clean Functions ---

def clean_dir(target_dir):
    """Removes all files and subdirectories within a target directory."""
    if not target_dir.exists():
        print(f"Directory {target_dir} does not exist, nothing to clean.")
        return
    print(f"Cleaning {target_dir}...")
    for item in target_dir.iterdir():
        try:
            if item.is_dir():
                # Don't remove the pdf subdir itself if cleaning html
                if target_dir == HTML_DIR and item.resolve() == PDF_DIR.resolve():
                    print(f"Skipping {item.name} directory.")
                    continue
                shutil.rmtree(item)
            else:
                item.unlink()
        except OSError as e:
            print(f"Error removing {item}: {e}")

def clean_html():
    """Cleans the HTML output directory, preserving the pdf subdirectory."""
    clean_dir(HTML_DIR)

def clean_pdf():
    """Cleans the PDF output directory."""
    clean_dir(PDF_DIR)

def clean_all():
    """Cleans both HTML and PDF output."""
    print("--- Cleaning All Output ---")
    # Clean PDF first as it's inside HTML
    clean_pdf()
    clean_html()
    # Optionally remove the active template copy
    if ACTIVE_TEMPLATE_DST.exists():
        print(f"Removing {ACTIVE_TEMPLATE_DST}...")
        ACTIVE_TEMPLATE_DST.unlink()
    print("--- Cleaning Complete ---")


# --- Main Execution ---

def main():
    parser = argparse.ArgumentParser(description="Build script for Typst project.")
    parser.add_argument(
        "target",
        nargs="?",
        default="html",
        choices=["html", "pdf", "clean", "clean-html", "clean-pdf", "copy-public", "serve", "update-id-path"],
        help="Build target or action (default: html)",
    )
    parser.add_argument(
        "-p", "--port",
        type=int,
        default=8000,
        help="Port to use for the serve command (default: 8000)"
    )

    args = parser.parse_args()

    if args.target == "html":
        write_id_to_path_map()
        build_html()
    elif args.target == "pdf":
        write_id_to_path_map()
        build_pdf()
    elif args.target == "clean":
        clean_all()
    elif args.target == "clean-html":
        clean_html()
    elif args.target == "clean-pdf":
        clean_pdf()
    elif args.target == "copy-public":
        copy_public_assets()
    elif args.target == "serve":
        serve_html(args.port)
    elif args.target == "update-id-path":
        write_id_to_path_map()

if __name__ == "__main__":
    main()
