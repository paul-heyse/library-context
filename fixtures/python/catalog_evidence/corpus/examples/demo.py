from pr3pkg import run, unseeded

state = []
state.append("before")
with open(__file__) as source:
    unseeded(source.read())
run(state[0])
