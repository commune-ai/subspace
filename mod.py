import commune as c
class Mod:
    """
    A base class that provides fundamental functionality for commune modules.
    """
    def __init__(self, **kwargs):
        """
        Initialize the base class with configurable parameters.
        Args:
            **kwargs: Arbitrary keyword arguments to configure the instance
        """
        # Store configuration as a Munch object for dot notation access
        self.config = c.munch(kwargs)
        self.model = c.module('openrouter')()

    def build(self, text="ignore the migrations", cmd='cargo build -p pallet-subspace', path='~/subspace/pallets/subspace/src'):
        return c.cmd(cmd)
        
    def forward(self, *text, cmd='cargo build -p pallet-subspace', path='~/subspace/pallets/subspace/src'):
        output = ''
        text = ' '.join(text)
        for ch in c.cmd(cmd):
            output += ch
            print(ch, end='')
        prompt = f'''
        {text}
        ---cmd--- 
        {cmd}
        --build---
        {output}
        --context--
        {c.file2text(path)}
        FIX THE ERRORS IN THE BUILD OUTPUT INCLUDE EVERYTHING IN THE BUILD OUTPUT
        '''
        return c.ask(prompt, process_text=False)
        
        