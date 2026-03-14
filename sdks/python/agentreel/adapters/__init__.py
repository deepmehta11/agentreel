"""Framework adapters for AgentReel.

Usage:
    # LangChain
    from agentreel.adapters.langchain import AgentReelCallbackHandler
    handler = AgentReelCallbackHandler(title="My LangChain run")
    chain.invoke(input, config={"callbacks": [handler]})
    handler.save("run.trajectory.json")

    # OpenAI Agents SDK
    from agentreel.adapters.openai_agents import AgentReelTracer
    tracer = AgentReelTracer(title="My agent run")
    # Use with agent hooks

    # CrewAI
    from agentreel.adapters.crewai import AgentReelCrewCallback
    crew = Crew(agents=[...], tasks=[...], callbacks=[AgentReelCrewCallback()])
"""
