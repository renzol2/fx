#include "moorer_verb.hpp"

#include <memory>

int main() {
  fx::MoorerReverb* pMoorerReverb = fx::create(44100);
  fx::set_damping(pMoorerReverb, 1.0);
  fx::set_frozen(pMoorerReverb, true);
  fx::set_wet(pMoorerReverb, 1.0);
  fx::set_width(pMoorerReverb, 1.0);
  fx::set_room_size(pMoorerReverb, 1.0);
  fx::destroy(pMoorerReverb);
  return 0;
}