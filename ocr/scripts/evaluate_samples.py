import argparse
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SAMPLES = ROOT / "samples"


def edit_distance(left, right):
    previous = list(range(len(right) + 1))
    for row, left_char in enumerate(left, start=1):
        current = [row]
        for column, right_char in enumerate(right, start=1):
            current.append(
                min(
                    current[column - 1] + 1,
                    previous[column] + 1,
                    previous[column - 1] + (left_char != right_char),
                )
            )
        previous = current
    return previous[-1]


def normalized(text):
    return " ".join(text.split())


def samples():
    for block in (SAMPLES / "ground-truth.txt").read_text().strip().split("\n\n"):
        filename, *truth_lines = block.splitlines()
        yield SAMPLES / filename, "\n".join(truth_lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--engine", default="tesseract")
    parser.add_argument("--language", default="eng")
    parser.add_argument("--tessdata-dir", type=Path)
    arguments = parser.parse_args()

    print("Image | Exact | CER | WER | Confidence | Elapsed")
    for image_path, truth in samples():
        command = [
            str(arguments.binary),
            "--engine",
            arguments.engine,
            "--language",
            arguments.language,
            str(image_path),
        ]
        if arguments.tessdata_dir:
            command[1:1] = ["--tessdata-dir", str(arguments.tessdata_dir)]
        completed = subprocess.run(command, check=True, capture_output=True, text=True)
        recognized, separator, details = completed.stdout.partition("\n\nConfidence: ")
        if not separator or not recognized.strip():
            raise RuntimeError(f"{image_path.name}: OCR returned no text or summary")

        expected = normalized(truth)
        actual = normalized(recognized)
        expected_words = expected.split()
        actual_words = actual.split()
        cer = 100 * edit_distance(expected, actual) / max(len(expected), 1)
        wer = 100 * edit_distance(expected_words, actual_words) / max(len(expected_words), 1)
        confidence = details.splitlines()[0]
        elapsed = re.search(r"Elapsed: ([0-9.]+ ms)", details)
        if not elapsed:
            raise RuntimeError(f"{image_path.name}: CLI omitted elapsed time")

        print(
            f"{image_path.name} | {actual == expected} | {cer:.1f}% | {wer:.1f}% | "
            f"{confidence} | {elapsed.group(1)}"
        )
        print(f"  expected: {truth.replace(chr(10), ' / ')}")
        print(f"  recognized: {recognized.replace(chr(10), ' / ').strip()}")


if __name__ == "__main__":
    main()
