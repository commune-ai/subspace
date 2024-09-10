import csv
from typing import List, Dict, Union

def calculate_average_black_box_age(file_path: str) -> float:
    black_box_ages: List[int] = []
    
    with open(file_path, 'r') as csvfile:
        csv_reader = csv.DictReader(csvfile)
        for row in csv_reader:
            black_box_age: int = int(row['Black Box Age'])
            black_box_ages.append(black_box_age)
    
    if not black_box_ages:
        raise ValueError("No data found in the CSV file")
    
    average_age: float = sum(black_box_ages) / len(black_box_ages)
    return average_age

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