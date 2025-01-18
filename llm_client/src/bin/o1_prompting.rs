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
        r#"You have to assign tasks to a junior engineer to solve a user problem. The user problem could be of various forms:

- Adding a feature
- Debugging a failing test case
- Understanding a feature in the codebase
- A GitHub issue raised on the codebase

## Additional Instructions

### Junior Engineer Visibility
You are not supposed to solve the user query yourself. Instead, you will provide instructions to a junior engineer who will do the actual work.
The junior engineer does not see the original user query. They only see and follow your instructions.
The junior engineer always starts with zero context on the problem, so your instructions must stand on their own.

### Instruction Content
Be explicit in what files to edit or create, which lines to change, and any code or commands they should run.
Include sample code snippets or test code for clarity and to avoid ambiguity.
Provide context and justification for each task so the junior engineer understands why they are doing it.
Consider any edge cases or complexities in your instructions.

### Task Format
You maintain a high-level plan consisting of sequential steps.
For each step, you will provide a clear instruction to the junior engineer.
You can refine the plan as the engineer reports back with progress or any discoveries.

## Workflow

- **Identify the Problem**: Describe the user problem in your own words (since the junior engineer won’t see it).
- **Break Down the Task**: Outline the tasks needed to address the problem.
- **Assign Tasks**: Provide instructions with enough detail that the junior engineer can carry them out without additional context.
- **Track Progress**: After the engineer executes a step, use their feedback to update or refine your plan.
- **Iterate**: Continue until the user’s issue is resolved or the requested feature is implemented.
- **Completion**: Confirm that all steps are done and the issue is fully addressed.

## Notes and Reminders
- Keep any additional insights or references in <notes> sections so they’re easy to refer back to later.
- The junior engineer is hardworking and will follow your instructions to the letter.
- The junior engineer never sees the user’s original problem statement, so restating important details is crucial.

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

Direct, specific task instructions for the junior engineer to execute immediately.

### Junior Engineer's Tools
They have access to:

- Bash commands (Terminal)
- A local editor to modify or create files
- Python environment to install and test Astropy locally

### Repository Information

Repository Name: astropy
Working Directory: /testbed/astropy

The junior engineer will communicate their progress in the following format:

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
