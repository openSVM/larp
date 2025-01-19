use llm_client::{
    clients::{
        openai::OpenAIClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{LLMProviderAPIKeys, OpenAIProvider},
};

#[tokio::main]
async fn main() {
    let developer_message = format!(
        r#"You have to assign tasks to a junior engineer to solve a Github Issue.
You will keep a high level plan and give out tasks to the junior engineer.
After the junior engineer has completed a task, they will report back to you, use that to further inform and improve your plan.
Keep refining the plan and giving out tasks to the junior engineer until the github issue is resolved.

## Rules to follow:
- You can not edit any test files nor are you allowed to edit the existing tests
- You can not create a new branch on the repository or change the commit of the repository.
- You are not allowed to run any tests.
- You should have a script which reproduces the reported issue. You might have to explore the repository to generate a really good script to recreate the issue.
- After making the changes in the codebase you should run the reproduction script again to make sure that the issue has been resolved.
- You cannot access any file outside the repository directory.
- You are not allowed to install any new packages as the developer environment has been already setup in the repository directory.

## How to leverage the junior engineer

### Junior Engineer Visibility
You are not supposed to solve the github issue yourself. Instead, you will provide instructions to a junior engineer who will do the actual work.
The junior engineer does not see the original github issue. They only work on the task you give them.
You are not supposed to write any code or work in the repository, use the junior engineer to perform the task instead.

### Junior engineer Instruction Content
Be explicit in what files to edit or create, what changes to make, and commands the junior engineer should run.
Include sample code snippets or test code for clarity and to avoid ambiguity.
Provide context and justification for each task so the junior engineer understands why they are doing it.
Consider any edge cases or complexities in your instructions.

## Plan generation

### Plan specifics
You maintain a high-level plan consisting of sequential instructions.
For each instruction, you will provide a clear task to the junior engineer.
You can refine the plan as the engineer reports back with progress or any discoveries.

## Workflow

- **Reproduce the Problem**: First reproduce the reported github issue a standalone python script to confirm the issue.
- **Identify the Problem**: Describe the github issue in your own words (since the junior engineer won’t see it).
- **Break Down the Task**: Outline the tasks needed to address the problem.
- **Assign Tasks**: Provide instructions with enough detail that the junior engineer can carry them out without additional context.
- **Track Progress**: After the engineer executes a task, use the generated artifacts (opened files, code changes, terminal output) to update or refine your plan.
- **Iterate**: Continue until the github issue is resolved.
- **Completion**: Confirm that the reproduction script solves the github issue and complete the task.

## Notes and Reminders
- Keep any additional insights or references in <notes> sections so they’re easy to refer back to later.
- You can use the <notes> along with the steps the junior engineer has taken for your instruction to plan out the next instruction for the junior engineer.

## Output Format Requirements

When you produce an output in response to the junior engineer's progress, include the following sections in this order:

### Plan Section

<plan>
<instruction>
{{High-level step-by-step plan}}
</instruction>
</plan>
This is the updated plan, reflecting the overall strategy and steps to address the user problem.

### Notes Section (if needed)

<notes>
{{Any helpful references, code snippets, or insights for future steps}}
</notes>
This can contain extra details or code for future use.

### Current Task Section

<current_task>
<instruction>
{{The specific instruction the engineer should execute next}}
</instruction>
</current_task>

Direct, specific standalone task instructions for the junior engineer to execute immediately.

### Junior Engineer's Tools
They have access to:

- Bash commands (Terminal)
- A local editor to modify or create files
- Python installed on the terminal to run standalone scripts

### Repository Information

Repository Name: astropy
Working Directory: /testbed/astropy

The junior engineer will communicate their progress after completing the instruction in the following format:

<current_instruction>
{{the instruction they are working on}}
</current_instruction>
And the steps they took to work on the instruction:
<steps>
<step>
<thinking>
{{engineer’s reasoning or approach}}
</thinking>
<tool_input>
{{commands or code they ran}}
</tool_input>
<tool_output>
{{results, errors, or logs}}
</tool_output>
</step>
</steps>"#
    );
    let user_message = format!(
        r#"<user_query>
Modeling's `separability_matrix` does not compute separability correctly for nested CompoundModels
Consider the following model:

```python
from astropy.modeling import models as m
from astropy.modeling.separable import separability_matrix

cm = m.Linear1D(10) & m.Linear1D(5)
```

It's separability matrix as you might expect is a diagonal:

```python
>>> separability_matrix(cm)
array([[ True, False],
        [False,  True]])
```

If I make the model more complex:
```python
>>> separability_matrix(m.Pix2Sky_TAN() & m.Linear1D(10) & m.Linear1D(5))
array([[ True,  True, False, False],
        [ True,  True, False, False],
        [False, False,  True, False],
        [False, False, False,  True]])
```

The output matrix is again, as expected, the outputs and inputs to the linear models are separable and independent of each other.

If however, I nest these compound models:
```python
>>> separability_matrix(m.Pix2Sky_TAN() & cm)
array([[ True,  True, False, False],
        [ True,  True, False, False],
        [False, False,  True,  True],
        [False, False,  True,  True]])
```
Suddenly the inputs and outputs are no longer separable?

This feels like a bug to me, but I might be missing something?
</user_query>"#
    );

    let llm_client = OpenAIClient::new();
    let completion_request = LLMClientCompletionRequest::new(
        LLMType::O1,
        vec![
            LLMClientMessage::system(developer_message),
            LLMClientMessage::user(user_message),
        ],
        0.2,
        None,
    );
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = llm_client
        .stream_completion(
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("".to_owned())),
            completion_request,
            sender,
        )
        .await
        .expect("to work");

    println!("response:\n{}", response.answer_up_until_now());
}
