# Contributing and Feedback Guide - Kore Package Manager (kpm)

[![Contribuye](https://img.shields.io/badge/Contribuye-aquí-green)](https://github.com/ezequielgk/Kore-Package-Manager/blob/main/CONTRIBUTING_es.md)

Thank you for your interest in improving **Kore Package Manager**! This is an open-source project, and all help is welcome, whether it's reporting bugs, suggesting improvements, or writing code.

## How to Help

There are several ways you can contribute to the development of `kpm`:

1.  **Report Bugs:** If something isn't working as it should, let us know.
2.  **Suggest Features:** Have a great idea? Share it!
3.  **Improve Documentation:** Fix typos or add useful examples.
4.  **Submit Pull Requests:** If you're a developer, you can clone the repository, make your changes, and submit them for review.

---

## How to Provide Feedback Correctly

Feedback is vital for the program's healthy growth. To make your feedback useful and easy for us to process, please follow these guidelines:

### 1. Where to leave feedback?
The best way is through **GitHub Issues** in the official repository.
*   **Bug Reports:** For reporting technical failures.
*   **Feature Requests:** For new ideas or design changes.

### 2. What to include in a good report?
To help us understand your issue or idea, we need context. Please include:

*   **kpm Version:** Run `kpm -V` and copy the output.
*   **Operating System:** Your Linux distribution (e.g., Arch Linux, Void, Fedora) and desktop environment (GNOME, KDE, etc.).
*   **Clear Description:** Explain what you were doing when the issue occurred or what you want to achieve with your suggestion.
*   **Steps to Reproduce (for bugs):**
    1. I ran `kpm install ...`
    2. I selected option X...
    3. The program crashed with error Y.
*   **Expected vs. Actual Behavior:** "What should have happened?" vs. "What actually happened?".
*   **Screenshots or Logs:** If the error occurs in the TUI, a screenshot helps a lot.

### 3. Be Courteous
Remember that this is a community-maintained project. Let's be kind and constructive in all our interactions.

---

## Do you want to help increase kpm packages? Do this!

If you know of an application distributed as a tarball that is not in our list, you can easily add it:

1.  **Fork** this repository.
2.  **Edit the file**: Open [`assets/community_repos.json`](file:///home/ezequiel/Documentos/Kore-Package-Manager/assets/community_repos.json).
3.  **Add the application**: Add a new JSON object to the end of the `repositories` list following this format:
    ```json
    {
      "name": "Visual Name",
      "package_name": "package-name",
      "url": "https://github.com/ezequielgk/Kore-Package-Manager",
      "category": "Utility",
      "requires_root": false,
      "terminal": false,
      "description": "A brief description of what the app does."
    }
    ```
    > **Note on URL**: We prefer direct links to repositories (e.g., GitHub). Do not use URLs to external pages; if it's necessary to use one, please explain it in the Pull Request chat.

4.  **Create a Pull Request**: Submit your changes with the title "Add package: [name]".

---

Thank you for being part of Kore Package Manager! 🦀
