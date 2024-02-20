# RGPT

Small terminal application to interact with chat-gpt via cmdline.

# Api-Key

You need an API-Key from OpenAI to use this.
The key must be saved as environment variable `OPENAI_KEY` to get picked up by rgpt.

So put this in your `.bashrc` or `.zshrc` or `.whatever-shell-you-have-rc`:
```shell
export OPENAI_KEY="sk-YOUR-ACTUAL-OPEN-AI-API-KEY"
```

# Usage

rgpt will parse your question from the standard input, if it detects no input arguments.
If you want, you can just type your question *after* the call to rgpt for direct usage.
As soon as rgpt detects that there are input arguments, it will parse all the words and interprets them as input
for chat-gpt:

```shell
rpgt why is rust an considered an inferior programming language by all the haters?
```

If you invoke rgpt without any arguments, it will prompt you for your input:
```shell
rpgt
Now, please tell me why nobody knows that rusts abstractions are also zero-cost ?
```
