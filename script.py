#!/usr/bin/env python3
import os
import re
import subprocess
import pandas as pd
import matplotlib.pyplot as plt

def parse_stats(output_text):
    """
    Extract statistics from Z3 output.
    Looks for lines with the pattern "key : value"
    and returns a dictionary of stats.
    """
    stats = {}
    for line in output_text.splitlines():
        line = line.strip()
        if not line:
            continue
        if ':' in line:
            key, value = line.split(":", 1)
            stats[key.strip()] = value.strip()
    return stats

def extract_numeric(stat_str):
    """
    Given a string, extract the first floating point number.
    Returns None if not found.
    """
    match = re.findall(r"[\d\.]+", stat_str)
    return float(match[0]) if match else None

def run_test(test_id):
    """
    Run Z3 on the SMT2 file for a given test ID with a 5-second timeout.
    Expects file in: studio/native/test/prg<id>/z3_chc_0/main.smt2
    Returns a dictionary with test stats.
    """
    directory = f"studio/native/test/prg{test_id}/z3_chc_0"
    file_path = os.path.join(directory, "main.smt2")
    if not os.path.exists(file_path):
        return {"Test": test_id, "Status": "File Not Found",
                "Solving Time (s)": None, "Conflicts": None,
                "Decisions": None, "Propagations": None}
    
    # Run Z3 with a timeout of 5 seconds
    command = ["z3", "-st", "-v:10", file_path]
    try:
        proc = subprocess.run(command, capture_output=True, text=True, timeout=5)
        output = proc.stdout
        stats = parse_stats(output)
        status = "OK"
        # Try to extract 'solving time' first, then fallback to 'time'
        solving_time = None
        for key in stats:
            if "solving time" in key:
                solving_time = extract_numeric(stats[key])
                break
        if solving_time is None and "time" in stats:
            solving_time = extract_numeric(stats["time"])
        # Extract additional stats if available
        conflicts = stats.get("conflicts", "N/A")
        decisions = stats.get("decisions", "N/A")
        propagations = stats.get("propagations", "N/A")
        return {"Test": test_id, "Status": status,
                "Solving Time (s)": solving_time, "Conflicts": conflicts,
                "Decisions": decisions, "Propagations": propagations}
    except subprocess.TimeoutExpired:
        # If the process does not finish within 5 seconds, mark as Timeout.
        return {"Test": test_id, "Status": "Timeout",
                "Solving Time (s)": None, "Conflicts": None,
                "Decisions": None, "Propagations": None}

def main():
    results = []
    total_tests = 55
    print("Running tests...\n")
    
    for test_id in range(1, total_tests + 1):
        result = run_test(test_id)
        print(f"Test {test_id}: {result['Status']}")
        results.append(result)
    
    # Create a table using pandas
    df = pd.DataFrame(results)
    print("\nStatistics Table:")
    print(df.to_string(index=False))
    
    # Plot a bar graph of solving times (only for tests that completed in time)
    df_time = df[df["Solving Time (s)"].notna()]
    if not df_time.empty:
        plt.figure(figsize=(10, 6))
        plt.bar(df_time["Test"], df_time["Solving Time (s)"])
        plt.xlabel("Test ID")
        plt.ylabel("Solving Time (s)")
        plt.title("Z3 Solving Time per Test")
        plt.xticks(range(1, total_tests + 1))
        plt.tight_layout()
        # Save the graph as an image file
        plt.savefig("stats_graph.png")
        plt.show()
    else:
        print("No solving time data available for plotting.")

if __name__ == "__main__":
    main()
