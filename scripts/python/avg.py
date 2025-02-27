import csv
from typing import List, Dict, Union



def main() -> None:
    file_path: str = '/root/subspace/tests/simulation_results.csv'
    try:
        average_age: float = calculate_average_black_box_age(file_path)
        print(f"The average Black Box Age is: {average_age:.2f}")
    except FileNotFoundError:
        print(f"Error: The file '{file_path}' was not found.")
    except ValueError as e:
        print(f"Error: {str(e)}")
    except Exception as e:
        print(f"An unexpected error occurred: {str(e)}")

if __name__ == "__main__":
    main()
