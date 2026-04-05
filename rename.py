import os
import re

def get_match_case_func(target_word):
    """
    Creates and returns a function to handle case matching dynamically
    based on the user's target word.
    """
    def match_case(match):
        word = match.group()
        if word.isupper():
            return target_word.upper()      # e.g., SEARCH -> TARGET
        elif word.istitle():
            return target_word.capitalize() # e.g., Search -> Target
        else:
            return target_word.lower()      # e.g., search -> target
    return match_case

def replace_everything(root_dir, search_word, target_word):
    # re.escape prevents errors if the search word contains special characters
    pattern = re.compile(re.escape(search_word), re.IGNORECASE)
    match_case_func = get_match_case_func(target_word)

    for root, dirs, files in os.walk(root_dir, topdown=False):

        # 1. Update file contents and file names
        for file in files:
            if file == 'rename_all.py':
                continue

            old_filepath = os.path.join(root, file)
            new_filepath = old_filepath

            # --- [A] Replace file content ---
            try:
                with open(old_filepath, 'r', encoding='utf-8') as f:
                    content = f.read()

                if pattern.search(content):
                    new_content = pattern.sub(match_case_func, content)
                    with open(old_filepath, 'w', encoding='utf-8') as f:
                        f.write(new_content)
                    print(f"📄 Content updated: {old_filepath}")
            except UnicodeDecodeError:
                pass # Ignore non-text files
            except Exception as e:
                print(f"⚠️ Content update error ({old_filepath}): {e}")

            # --- [B] Rename file ---
            if pattern.search(file):
                new_filename = pattern.sub(match_case_func, file)
                new_filepath = os.path.join(root, new_filename)
                os.rename(old_filepath, new_filepath)
                print(f"🔄 File renamed: {file} -> {new_filename}")

        # 2. Rename directories
        for dirname in dirs:
            if dirname in ['.git', '__pycache__', 'node_modules', '.venv']:
                continue

            if pattern.search(dirname):
                new_dirname = pattern.sub(match_case_func, dirname)
                old_dirpath = os.path.join(root, dirname)
                new_dirpath = os.path.join(root, new_dirname)
                os.rename(old_dirpath, new_dirpath)
                print(f"📁 Directory renamed: {dirname} -> {new_dirname}")


if __name__ == "__main__":
    print("=== Batch Rename & Replace Tool ===")

    # Get user inputs
    search_input = input("Enter the word to find: ")
    target_input = input("Enter the word to replace it with: ")
    folder_input = input("Enter the target directory path (press Enter for current '.'): ")

    # Default to current directory if nothing is entered
    if not folder_input.strip():
        folder_input = '.'

    print(f"\n🚀 Starting to replace '{search_input}' with '{target_input}'...\n")

    replace_everything(folder_input, search_input, target_input)

    print("\n✅ All tasks completed successfully!")