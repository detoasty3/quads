#include <iostream>
#include <vector>
#include <algorithm>
#include <queue>
#include <stack>
#include <deque>
#include <set>
#include <unordered_set>
#include <map>
#include <unordered_map>
#include <string>
#include <cmath>
#include <iomanip>
#include <climits>
#include <bitset>
 
typedef unsigned long long ull;
typedef long long ll;
 
using namespace std;

void iterate(int *a, int i, int size, int n, ll &count) {
    int start = 0;
    if (i > 0) {
        start = a[i-1]+1;
    }
    for (a[i] = start; a[i] < size; a[i]++) {
        if (i == n-1) {
            vector<int> subset;
            for (int k = 0; k < n; k++) {
                subset.push_back(a[k]);
            }
            ll curcount = 0;
            for (int i = 0; i < n; i++) {
                for (int j = i+1; j < n; j++) {
                    for (int k = j+1; k < n; k++) {
                        for (int l = k+1; l < n; l++) {
                            int cur = 0;
                            cur ^= subset[i];
                            cur ^= subset[j];
                            cur ^= subset[k];
                            cur ^= subset[l];
                            if (cur == 0) {
                                curcount++;
                            }
                            count = max(count, curcount);
                        }
                    }
                }
            }
        }
        else {
            iterate(a, i+1, size, n, count);
        }
    }
}
 
int main() {
    int a[64];
    for (int i = 0; i < 64; i++) {
        a[i] = i;
    }
    ll count = 0;
    // Line 68 may be adjusted to anything of the form "iterate(a, 0, 2^{2d}, k, count);"
    // d is the number of dimensions (i.e., the number of card properties)
    // k is the size of the subset of cards chosen from a deck of 2^{2d} cards
    // Line 68 will find q(7, 6)
    iterate(a, 0, 64, 7, count);
    cout << count << endl;
}