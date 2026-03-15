import re

with open('cuda/base_neuron.cu', 'r') as f:
    code = f.read()

code = re.sub(
    r'    void tick_neuron_prepare_cpu',
    '''    void tick_neuron_cpu(BaseNeuron* neuron, BaseNeuron* all_neurons, int count, Environment env, uint32_t current_tick) {
        (void)all_neurons;
        (void)count;
        tick_neuron_prepare_cpu(neuron, env, current_tick);
        tick_neuron_spike_cpu(neuron, env, current_tick);
    }
    
    void tick_neuron_prepare_cpu''',
    code
)

with open('cuda/base_neuron.cu', 'w') as f:
    f.write(code)

